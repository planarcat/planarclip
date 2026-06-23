use std::io::ErrorKind;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use crate::crypto::keys::{peer_id_from_public_key, KeyPair};
use crate::network::protocol::{HandshakeMessage, SignalMessage};

const FRAME_HANDSHAKE: u8 = 0x00;
const FRAME_DATA: u8 = 0x01;

pub enum Frame {
    Handshake(HandshakeMessage),
    Data(SignalMessage),
}

pub async fn read_frame(reader: &mut (impl AsyncRead + Unpin)) -> Result<Frame, FrameError> {
    let mut header = [0u8; 5];
    reader.read_exact(&mut header).await?;

    let frame_type = header[0];
    let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

    if len > 16 * 1024 * 1024 {
        return Err(FrameError::PayloadTooLarge(len));
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;

    match frame_type {
        FRAME_HANDSHAKE => Ok(Frame::Handshake(serde_json::from_slice(&payload)?)),
        FRAME_DATA => Ok(Frame::Data(serde_json::from_slice(&payload)?)),
        other => Err(FrameError::UnknownFrameType(other)),
    }
}

pub async fn write_frame(writer: &mut (impl AsyncWrite + Unpin), frame: &Frame) -> Result<(), FrameError> {
    let (frame_type, json) = match frame {
        Frame::Handshake(msg) => (FRAME_HANDSHAKE, serde_json::to_vec(msg)?),
        Frame::Data(msg) => (FRAME_DATA, serde_json::to_vec(msg)?),
    };

    let len = json.len() as u32;
    let mut buf = Vec::with_capacity(5 + json.len());
    buf.push(frame_type);
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&json);

    writer.write_all(&buf).await?;
    Ok(())
}

pub async fn tcp_connect(ip: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    let stream = TcpStream::connect(format!("{}:{}", ip, port)).await?;
    stream.set_nodelay(true)?;
    Ok(stream)
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("I/O 错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 错误：{0}")]
    Json(#[from] serde_json::Error),
    #[error("负载过大：{0} 字节")]
    PayloadTooLarge(usize),
    #[error("未知帧类型：0x{0:02x}")]
    UnknownFrameType(u8),
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("I/O 错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("帧错误：{0}")]
    Frame(#[from] FrameError),
    #[error("对方已拒绝连接")]
    Rejected,
    #[error("配对码无效")]
    InvalidCode,
    #[error("配对码已过期")]
    Timeout,
    #[error("用户已取消")]
    Cancelled,
    #[error("协议错误：{0}")]
    Protocol(&'static str),
}

impl HandshakeError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::InvalidCode => "invalid_code",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Protocol(_) | Self::Frame(_) => "protocol_error",
            Self::Io(error) => match error.kind() {
                ErrorKind::UnexpectedEof
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::BrokenPipe => "connection_lost",
                _ => "connection_unavailable",
            },
        }
    }

    pub fn user_message(&self) -> String {
        match self.reason_code() {
            "rejected" => "对方已拒绝这次连接。".into(),
            "invalid_code" => "配对码不正确，请重新核对后再试。".into(),
            "timeout" => "这次连接已超时，请重新发起连接。".into(),
            "cancelled" => "这次连接已取消，请重新发起连接。".into(),
            "connection_lost" => "对方设备已断开连接，请重新发起连接。".into(),
            "protocol_error" => "连接过程中出了点问题，请重新发起连接。".into(),
            _ => "暂时无法连接对方设备，请确认对方应用已打开，而且你们在同一局域网内。".into(),
        }
    }

    fn from_reason_code(reason: Option<&str>) -> Self {
        match reason {
            Some("invalid_code") => Self::InvalidCode,
            Some("timeout") => Self::Timeout,
            Some("cancelled") => Self::Cancelled,
            Some("rejected") => Self::Rejected,
            Some("protocol_error") => Self::Protocol("协议状态异常"),
            _ => Self::Rejected,
        }
    }
}

pub struct DirectConnection {
    pub rx: mpsc::UnboundedReceiver<SignalMessage>,
    pub tx: mpsc::UnboundedSender<String>,
    pub peer_name: String,
    pub peer_id: String,
    pub peer_public_key: Vec<u8>,
}

fn spawn_data_bridge(
    stream: TcpStream,
    peer_name: String,
    peer_id: String,
    peer_public_key: Vec<u8>,
) -> DirectConnection {
    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<SignalMessage>();
    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<String>();

    let (mut read_half, mut write_half) = stream.into_split();

    tokio::spawn(async move {
        loop {
            match read_frame(&mut read_half).await {
                Ok(Frame::Data(msg)) => {
                    if incoming_tx.send(msg).is_err() {
                        break;
                    }
                }
                Ok(Frame::Handshake(_)) => {
                    tracing::warn!("Unexpected handshake frame on data channel; ignoring");
                }
                Err(e) => {
                    tracing::warn!("TCP read closed: {}", e);
                    break;
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some(json) = outgoing_rx.recv().await {
            match serde_json::from_str::<SignalMessage>(&json) {
                Ok(msg) => {
                    if write_frame(&mut write_half, &Frame::Data(msg)).await.is_err() {
                        tracing::warn!("TCP write closed");
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("Invalid outgoing message: {}", e);
                }
            }
        }
    });

    DirectConnection {
        rx: incoming_rx,
        tx: outgoing_tx,
        peer_name,
        peer_id,
        peer_public_key,
    }
}

pub enum InitiatorResult {
    Connected(DirectConnection),
    AwaitingCode { stream: TcpStream },
}

pub async fn initiator_connect(
    ip: &str,
    port: u16,
    device_name: &str,
    key_pair: &KeyPair,
) -> Result<InitiatorResult, HandshakeError> {
    let mut stream = tcp_connect(ip, port).await?;

    let req = HandshakeMessage::ConnectRequest {
        device_name: device_name.to_string(),
        peer_id: key_pair.fingerprint(),
        public_key: hex::encode(key_pair.public_bytes()),
    };
    write_frame(&mut stream, &Frame::Handshake(req)).await?;

    match read_frame(&mut stream).await? {
        Frame::Handshake(HandshakeMessage::AuthResult {
            success: true,
            peer_name,
            public_key,
            ..
        }) => {
            let name = peer_name.unwrap_or_default();
            let pk_bytes = public_key
                .as_deref()
                .and_then(|h| hex::decode(h).ok())
                .unwrap_or_default();
            let pid = short_fingerprint(&pk_bytes);
            Ok(InitiatorResult::Connected(spawn_data_bridge(stream, name, pid, pk_bytes)))
        }
        Frame::Handshake(HandshakeMessage::AwaitCode) => Ok(InitiatorResult::AwaitingCode { stream }),
        Frame::Handshake(HandshakeMessage::AuthResult {
            success: false,
            reason,
            ..
        }) => Err(HandshakeError::from_reason_code(reason.as_deref())),
        _ => Err(HandshakeError::Protocol(
            "expected AuthResult or AwaitCode after ConnectRequest",
        )),
    }
}

pub async fn initiator_send_code(
    mut stream: TcpStream,
    code: String,
) -> Result<DirectConnection, HandshakeError> {
    write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AuthCode { code })).await?;

    match read_frame(&mut stream).await? {
        Frame::Handshake(HandshakeMessage::AuthResult {
            success: true,
            peer_name,
            public_key,
            ..
        }) => {
            let name = peer_name.unwrap_or_default();
            let pk_bytes = public_key
                .as_deref()
                .and_then(|h| hex::decode(h).ok())
                .unwrap_or_default();
            let pid = short_fingerprint(&pk_bytes);
            Ok(spawn_data_bridge(stream, name, pid, pk_bytes))
        }
        Frame::Handshake(HandshakeMessage::AuthResult {
            success: false,
            reason,
            ..
        }) => Err(HandshakeError::from_reason_code(reason.as_deref())),
        _ => Err(HandshakeError::Protocol("expected AuthResult after AuthCode")),
    }
}

pub struct IncomingRequest {
    pub stream: TcpStream,
    pub initiator_name: String,
    pub initiator_peer_id: String,
    pub initiator_public_key: Vec<u8>,
}

pub async fn read_connect_request(mut stream: TcpStream) -> Result<IncomingRequest, HandshakeError> {
    let frame = read_frame(&mut stream).await?;
    match frame {
        Frame::Handshake(HandshakeMessage::ConnectRequest {
            device_name,
            peer_id,
            public_key,
        }) => {
            let pk_bytes = hex::decode(&public_key)
                .map_err(|_| HandshakeError::Protocol("公钥十六进制格式无效"))?;
            Ok(IncomingRequest {
                stream,
                initiator_name: device_name,
                initiator_peer_id: peer_id,
                initiator_public_key: pk_bytes,
            })
        }
        _ => Err(HandshakeError::Protocol("应收到连接请求消息")),
    }
}

pub async fn responder_accept_trusted(
    mut stream: TcpStream,
    device_name: &str,
    key_pair: &KeyPair,
    initiator_name: String,
    initiator_public_key: Vec<u8>,
) -> Result<DirectConnection, HandshakeError> {
    let pk_hex = hex::encode(key_pair.public_bytes());
    write_frame(
        &mut stream,
        &Frame::Handshake(HandshakeMessage::AuthResult {
            success: true,
            peer_name: Some(device_name.to_string()),
            public_key: Some(pk_hex),
            reason: None,
        }),
    )
    .await?;

    let pid = short_fingerprint(&initiator_public_key);
    Ok(spawn_data_bridge(stream, initiator_name, pid, initiator_public_key))
}

pub async fn responder_verify_code(
    mut stream: TcpStream,
    device_name: &str,
    key_pair: &KeyPair,
    initiator_name: String,
    initiator_public_key: Vec<u8>,
    pairing_code: &str,
    reject_rx: oneshot::Receiver<()>,
) -> Result<DirectConnection, HandshakeError> {
    write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AwaitCode)).await?;
    let received_code = tokio::select! {
        frame = read_frame(&mut stream) => {
            match frame? {
                Frame::Handshake(HandshakeMessage::AuthCode { code }) => code,
                _ => return Err(HandshakeError::Protocol("应收到配对码消息")),
            }
        }
        _ = reject_rx => {
            let _ = write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AuthResult {
                success: false,
                peer_name: None,
                public_key: None,
                reason: Some("rejected".into()),
            })).await;
            return Err(HandshakeError::Cancelled);
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
            let _ = write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AuthResult {
                success: false,
                peer_name: None,
                public_key: None,
                reason: Some("timeout".into()),
            })).await;
            return Err(HandshakeError::Timeout);
        }
    };

    if received_code != pairing_code {
        write_frame(
            &mut stream,
            &Frame::Handshake(HandshakeMessage::AuthResult {
                success: false,
                peer_name: None,
                public_key: None,
                reason: Some("invalid_code".into()),
            }),
        )
        .await?;
        return Err(HandshakeError::InvalidCode);
    }

    let pk_hex = hex::encode(key_pair.public_bytes());
    write_frame(
        &mut stream,
        &Frame::Handshake(HandshakeMessage::AuthResult {
            success: true,
            peer_name: Some(device_name.to_string()),
            public_key: Some(pk_hex),
            reason: None,
        }),
    )
    .await?;

    let pid = short_fingerprint(&initiator_public_key);
    Ok(spawn_data_bridge(stream, initiator_name, pid, initiator_public_key))
}

pub enum ListenerEvent {
    Incoming(IncomingRequest),
}

pub async fn run_listener(
    port: u16,
    event_tx: mpsc::UnboundedSender<ListenerEvent>,
) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("TCP listener bound to port {}", port);

    loop {
        let (stream, addr) = listener.accept().await?;
        tracing::info!("TCP connection accepted from {}", addr);

        let tx = event_tx.clone();
        tokio::spawn(async move {
            match read_connect_request(stream).await {
                Ok(req) => {
                    let _ = tx.send(ListenerEvent::Incoming(req));
                }
                Err(e) => {
                    tracing::warn!("Failed to read ConnectRequest from {}: {}", addr, e);
                }
            }
        });
    }
}

pub(crate) fn short_fingerprint(pk_bytes: &[u8]) -> String {
    peer_id_from_public_key(pk_bytes)
}

pub(crate) fn generate_pairing_code() -> String {
    use rand::Rng;
    format!("{:06}", rand::thread_rng().gen_range(0..1_000_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn bind_test_listener() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    #[tokio::test]
    async fn trusted_peer_is_auto_accepted() {
        let (listener, port) = bind_test_listener().await;
        let initiator_key = KeyPair::generate();
        let responder_key = KeyPair::generate();
        let responder_key_for_server = responder_key.clone();

        // 先让发起方连上，再在服务端读取请求并完成握手，避免测试里的 accept 时序互相卡住。
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let request = read_connect_request(stream).await.unwrap();
            responder_accept_trusted(
                request.stream,
                "Responder",
                &responder_key_for_server,
                request.initiator_name,
                request.initiator_public_key,
            )
            .await
            .unwrap()
        });

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key)
            .await
            .unwrap();
        let connection = match result {
            InitiatorResult::Connected(connection) => connection,
            InitiatorResult::AwaitingCode { .. } => panic!("trusted peer should connect directly"),
        };

        assert_eq!(connection.peer_name, "Responder");
        assert_eq!(connection.peer_id, short_fingerprint(&responder_key.public_bytes()));

        let server_connection = server.await.unwrap();
        assert_eq!(server_connection.peer_name, "Initiator");
        assert_eq!(server_connection.peer_id, short_fingerprint(&initiator_key.public_bytes()));
    }

    #[tokio::test]
    async fn pairing_code_flow_connects_unknown_peer() {
        let (listener, port) = bind_test_listener().await;
        let initiator_key = KeyPair::generate();
        let responder_key = KeyPair::generate();
        let responder_key_for_server = responder_key.clone();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let request = read_connect_request(stream).await.unwrap();
            let (_reject_tx, reject_rx) = oneshot::channel();
            responder_verify_code(
                request.stream,
                "Responder",
                &responder_key_for_server,
                request.initiator_name,
                request.initiator_public_key,
                "123456",
                reject_rx,
            )
            .await
            .unwrap()
        });

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key)
            .await
            .unwrap();
        let stream = match result {
            InitiatorResult::AwaitingCode { stream } => stream,
            InitiatorResult::Connected(_) => panic!("unknown peer should require pairing code"),
        };

        let connection = initiator_send_code(stream, "123456".into()).await.unwrap();
        assert_eq!(connection.peer_name, "Responder");
        assert_eq!(connection.peer_id, short_fingerprint(&responder_key.public_bytes()));

        let server_connection = server.await.unwrap();
        assert_eq!(server_connection.peer_name, "Initiator");
        assert_eq!(server_connection.peer_id, short_fingerprint(&initiator_key.public_bytes()));
    }

    #[tokio::test]
    async fn invalid_pairing_code_is_reported_to_both_sides() {
        let (listener, port) = bind_test_listener().await;
        let initiator_key = KeyPair::generate();
        let responder_key = KeyPair::generate();
        let responder_key_for_server = responder_key.clone();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let request = read_connect_request(stream).await.unwrap();
            let (_reject_tx, reject_rx) = oneshot::channel();
            responder_verify_code(
                request.stream,
                "Responder",
                &responder_key_for_server,
                request.initiator_name,
                request.initiator_public_key,
                "123456",
                reject_rx,
            )
            .await
        });

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key)
            .await
            .unwrap();
        let stream = match result {
            InitiatorResult::AwaitingCode { stream } => stream,
            InitiatorResult::Connected(_) => panic!("unknown peer should require pairing code"),
        };

        let client_result = initiator_send_code(stream, "654321".into()).await;
        assert!(matches!(client_result, Err(HandshakeError::InvalidCode)));

        let server_result = server.await.unwrap();
        assert!(matches!(server_result, Err(HandshakeError::InvalidCode)));
    }
}
