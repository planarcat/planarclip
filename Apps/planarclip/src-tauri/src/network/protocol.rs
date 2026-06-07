use serde::{Deserialize, Serialize};

/// Messages exchanged between peers via the signalling WebSocket relay.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalMessage {
    /// Clipboard text sync: payload + full BLAKE3 hash (hex-encoded, 64 chars)
    #[serde(rename = "clipboard")]
    Clipboard {
        payload: String,
        /// Full 32-byte BLAKE3 hash of payload, hex-encoded (64 hex chars).
        hash: String,
    },

    /// Server notification: another peer joined the room.
    #[serde(rename = "peer_joined")]
    PeerJoined {
        peer_id: String,
    },

    /// Server notification: another peer left the room.
    #[serde(rename = "peer_left")]
    PeerLeft {
        peer_id: String,
    },
}

/// Messages exchanged during TCP direct-connection handshake (before encryption).
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HandshakeMessage {
    /// Initiator → Responder: request to establish a connection.
    #[serde(rename = "connect_request")]
    ConnectRequest {
        device_name: String,
        /// Short fingerprint of the initiator's public key for display.
        peer_id: String,
        /// Initiator's X25519 public key, hex-encoded (64 hex chars).
        public_key: String,
    },

    /// Responder → Initiator: pairing code required — show the code input UI.
    /// Only sent when the initiator is not a trusted peer.
    #[serde(rename = "await_code")]
    AwaitCode,

    /// Initiator → Responder: submit the 6-digit pairing code.
    #[serde(rename = "auth_code")]
    AuthCode {
        code: String,
    },

    /// Responder → Initiator: result of the pairing code verification.
    #[serde(rename = "auth_result")]
    AuthResult {
        success: bool,
        /// Responder's device name (set on success).
        peer_name: Option<String>,
        /// Responder's X25519 public key, hex-encoded (set on success).
        public_key: Option<String>,
    },
}
