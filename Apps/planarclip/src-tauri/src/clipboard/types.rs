use std::io::Write;

#[derive(Debug, Clone)]
pub enum ClipboardSnapshot {
    Text(String),
    Empty,
}

impl ClipboardSnapshot {
    pub fn content_hash(&self) -> [u8; 32] {
        match self {
            ClipboardSnapshot::Text(s) => *blake3::hash(s.as_bytes()).as_bytes(),
            ClipboardSnapshot::Empty => [0u8; 32],
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            ClipboardSnapshot::Text(text) => Some(text),
            ClipboardSnapshot::Empty => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClipboardOrigin {
    Local,
    Remote { peer_name: String },
}

#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    pub snapshot: ClipboardSnapshot,
    pub origin: ClipboardOrigin,
    pub timestamp_ms: u64,
}

impl ClipboardEvent {
    pub fn local(snapshot: ClipboardSnapshot) -> Self {
        Self {
            snapshot,
            origin: ClipboardOrigin::Local,
            timestamp_ms: now_ms(),
        }
    }

    pub fn remote(snapshot: ClipboardSnapshot, peer_name: String) -> Self {
        Self {
            snapshot,
            origin: ClipboardOrigin::Remote { peer_name },
            timestamp_ms: now_ms(),
        }
    }

    pub fn content_hash(&self) -> [u8; 32] {
        self.snapshot.content_hash()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClipboardHistoryEntry {
    pub id: String,
    pub content: String,
    pub source_label: String,
    pub direction: String,
    pub timestamp_ms: u64,
}

pub fn debug_report(hypothesis_id: &str, location: &str, msg: &str, data: serde_json::Value) {
    let env_path = std::path::Path::new(".dbg/clipboard-history-duplicate.env");
    let mut debug_server_url = "http://127.0.0.1:7777/event".to_string();
    let mut session_id = "clipboard-history-duplicate".to_string();

    if let Ok(contents) = std::fs::read_to_string(env_path) {
        for line in contents.lines() {
            if let Some(value) = line.strip_prefix("DEBUG_SERVER_URL=") {
                debug_server_url = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("DEBUG_SESSION_ID=") {
                session_id = value.trim().to_string();
            }
        }
    }

    let Some((authority, path)) = debug_server_url
        .trim_start_matches("http://")
        .split_once('/')
    else {
        return;
    };

    let payload = serde_json::json!({
        "sessionId": session_id,
        "runId": "post-fix",
        "hypothesisId": hypothesis_id,
        "location": location,
        "msg": msg,
        "data": data,
        "ts": now_ms(),
    });

    let Ok(body) = serde_json::to_vec(&payload) else {
        return;
    };

    if let Ok(mut stream) = std::net::TcpStream::connect(authority) {
        let request = format!(
            "POST /{} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            path,
            authority,
            body.len()
        );
        let _ = stream.write_all(request.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
