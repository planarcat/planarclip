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
