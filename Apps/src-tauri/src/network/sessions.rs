use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::network::webrtc::ConnectionHandle;
use crate::ConnectedPeerPayload;

pub struct ConnectionSession {
    pub handle: ConnectionHandle,
    pub peer_name: String,
    pub session_generation: u64,
    pub connected: Arc<Mutex<bool>>,
}

pub struct ConnectionRegistry {
    sessions: HashMap<String, ConnectionSession>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn contains(&self, peer_id: &str) -> bool {
        self.sessions.contains_key(peer_id)
    }

    pub fn insert(&mut self, peer_id: String, session: ConnectionSession) {
        tracing::info!(
            target: "connection",
            peer_id = %crate::logging::redact_peer(&peer_id),
            peer_name = %session.peer_name,
            generation = session.session_generation,
            "connection established"
        );
        self.sessions.insert(peer_id, session);
    }

    pub fn get(&self, peer_id: &str) -> Option<&ConnectionSession> {
        self.sessions.get(peer_id)
    }

    pub fn remove(&mut self, peer_id: &str) -> Option<ConnectionSession> {
        let session = self.sessions.remove(peer_id);
        if let Some(session) = session.as_ref() {
            tracing::info!(
                target: "connection",
                peer_id = %crate::logging::redact_peer(peer_id),
                peer_name = %session.peer_name,
                "connection removed"
            );
        }
        session
    }

    pub fn drain_all(&mut self) -> Vec<(String, ConnectionSession)> {
        let drained: Vec<_> = self.sessions.drain().collect();
        tracing::info!(target: "connection", count = drained.len(), "all connections drained");
        drained
    }

    pub fn connected_peer_ids(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    pub fn connected_peers(&self) -> Vec<ConnectedPeerPayload> {
        self.sessions
            .iter()
            .map(|(peer_id, session)| ConnectedPeerPayload {
                peer_id: peer_id.clone(),
                peer_name: session.peer_name.clone(),
            })
            .collect()
    }

    pub fn active_handles(&self) -> Vec<ConnectionHandle> {
        self.sessions
            .values()
            .filter(|session| {
                session
                    .connected
                    .try_lock()
                    .map(|guard| *guard)
                    .unwrap_or(false)
            })
            .map(|session| session.handle.clone())
            .collect()
    }
}
