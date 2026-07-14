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
        #[serde(default)]
        batch_bytes_total: Option<u64>,
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


// ---- inline unit tests ----
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_message_clipboard_roundtrip() {
        // 帧编解码回环：Rust <-> JSON <-> Rust 必须无损，
        // 这是双端保持协议兼容的基本前提。
        let msg = SignalMessage::Clipboard {
            payload: "hello 你好".to_string(),
            hash: "deadbeef".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"clipboard\""));
        assert!(json.contains("\"hello 你好\""));

        let decoded: SignalMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            SignalMessage::Clipboard { payload, hash } => {
                assert_eq!(payload, "hello 你好");
                assert_eq!(hash, "deadbeef");
            }
            _ => panic!("expected Clipboard variant"),
        }
    }

    #[test]
    fn signal_message_file_batch_end_tag_matches_wire_name() {
        let msg = SignalMessage::ClipboardFileBatchEnd {
            batch_id: "b1".into(),
            file_count: 3,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"clipboard_file_batch_end\""));
    }

    #[test]
    fn transfer_cancel_reason_is_optional() {
        // reason 缺省时应能反序列化，避免旧版发送方不带 reason 时崩溃
        let json = r#"{"type":"transfer_cancel","transfer_id":"t1"}"#;
        let decoded: SignalMessage = serde_json::from_str(json).unwrap();
        match decoded {
            SignalMessage::TransferCancel { transfer_id, reason } => {
                assert_eq!(transfer_id, "t1");
                assert!(reason.is_none());
            }
            _ => panic!("expected TransferCancel"),
        }
    }

    #[test]
    fn handshake_presence_query_and_reply_wire_names_are_stable() {
        // Presence 探测的 wire type 名称是跨版本契约（见 presence-probe 主题）
        let query = HandshakeMessage::PresenceQuery {
            querier_peer_id: "abcd".into(),
        };
        let reply = HandshakeMessage::PresenceReply {
            peer_id: "abcd".into(),
            device_name: "planarcat".into(),
            service_profile: "release".into(),
        };
        assert!(serde_json::to_string(&query).unwrap().contains("\"type\":\"presence_query\""));
        assert!(serde_json::to_string(&reply).unwrap().contains("\"type\":\"presence_reply\""));
    }

    #[test]
    fn handshake_connect_request_requires_confirmation_default_false() {
        // 缺省 requires_confirmation 必须解析为 false，保证旧发起方兼容
        let json = r#"{
            "type":"connect_request",
            "device_name":"A",
            "peer_id":"pa",
            "public_key":"aa"
        }"#;
        let decoded: HandshakeMessage = serde_json::from_str(json).unwrap();
        match decoded {
            HandshakeMessage::ConnectRequest { requires_confirmation, .. } => {
                assert!(!requires_confirmation);
            }
            _ => panic!("expected ConnectRequest"),
        }
    }

    #[test]
    fn auth_result_success_serialization_omits_none_fields_readably() {
        // 序列化后即使字段是 null 也不能改变 tag 名
        let msg = HandshakeMessage::AuthResult {
            success: true,
            peer_name: Some("A".into()),
            public_key: None,
            reason: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"auth_result\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn clipboard_file_meta_item_roundtrip() {
        let item = ClipboardFileMetaItem {
            file_name: "readme.md".into(),
            size_bytes: 42,
            content_hash: "cafebabe".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: ClipboardFileMetaItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.file_name, "readme.md");
        assert_eq!(decoded.size_bytes, 42);
        assert_eq!(decoded.content_hash, "cafebabe");
    }
}
