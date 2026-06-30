use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalMessage {
    #[serde(rename = "clipboard")]
    Clipboard {
        payload: String,
        hash: String,
    },

    #[serde(rename = "clipboard_image_inline")]
    ClipboardImageInline {
        hash: String,
        width: u32,
        height: u32,
        mime: String,
        data_base64: String,
    },

    #[serde(rename = "clipboard_image_begin")]
    ClipboardImageBegin {
        transfer_id: String,
        hash: String,
        width: u32,
        height: u32,
        total_bytes: u64,
        chunk_size: u32,
    },

    #[serde(rename = "clipboard_image_end")]
    ClipboardImageEnd {
        transfer_id: String,
        hash: String,
    },

    #[serde(rename = "clipboard_file_begin")]
    ClipboardFileBegin {
        transfer_id: String,
        hash: String,
        file_name: String,
        total_bytes: u64,
        chunk_size: u32,
        batch_id: Option<String>,
        batch_index: Option<u32>,
        batch_total: Option<u32>,
    },

    #[serde(rename = "clipboard_file_end")]
    ClipboardFileEnd {
        transfer_id: String,
        hash: String,
    },

    #[serde(rename = "clipboard_file_batch_end")]
    ClipboardFileBatchEnd {
        batch_id: String,
        file_count: u32,
    },

    /// File names and sizes only — no binary payload (used when file sync is disabled).
    #[serde(rename = "clipboard_file_list_meta")]
    ClipboardFileListMeta {
        hash: String,
        files: Vec<ClipboardFileMetaItem>,
    },

    #[serde(rename = "transfer_ack")]
    TransferAck {
        transfer_id: String,
        chunk_index: u32,
    },

    #[serde(rename = "transfer_cancel")]
    TransferCancel {
        transfer_id: String,
        reason: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardFileMetaItem {
    pub file_name: String,
    pub size_bytes: u64,
    pub content_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HandshakeMessage {
    #[serde(rename = "connect_request")]
    ConnectRequest {
        device_name: String,
        peer_id: String,
        public_key: String,
        /// When true, the initiator does not yet treat the target as a familiar device and
        /// the responder must not auto-accept even if the initiator is locally trusted.
        #[serde(default)]
        requires_confirmation: bool,
    },

    #[serde(rename = "await_code")]
    AwaitCode,

    #[serde(rename = "auth_code")]
    AuthCode {
        code: String,
    },

    #[serde(rename = "local_pairing_code")]
    LocalPairingCode {
        code: String,
    },

    #[serde(rename = "auth_result")]
    AuthResult {
        success: bool,
        peer_name: Option<String>,
        public_key: Option<String>,
        reason: Option<String>,
    },

    #[serde(rename = "presence_query")]
    PresenceQuery {
        querier_peer_id: String,
    },

    #[serde(rename = "presence_reply")]
    PresenceReply {
        peer_id: String,
        device_name: String,
        service_profile: String,
    },
}
