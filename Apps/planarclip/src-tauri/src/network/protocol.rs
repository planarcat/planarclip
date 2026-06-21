use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalMessage {
    #[serde(rename = "clipboard")]
    Clipboard {
        payload: String,
        hash: String,
    },

    #[serde(rename = "peer_joined")]
    PeerJoined {
        peer_id: String,
    },

    #[serde(rename = "peer_left")]
    PeerLeft {
        peer_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HandshakeMessage {
    #[serde(rename = "connect_request")]
    ConnectRequest {
        device_name: String,
        peer_id: String,
        public_key: String,
    },

    #[serde(rename = "await_code")]
    AwaitCode,

    #[serde(rename = "auth_code")]
    AuthCode {
        code: String,
    },

    #[serde(rename = "auth_result")]
    AuthResult {
        success: bool,
        peer_name: Option<String>,
        public_key: Option<String>,
        reason: Option<String>,
    },
}
