use std::io::ErrorKind;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use crate::crypto::keys::{peer_id_from_public_key, KeyPair};
use crate::network::binary_chunk::{decode_binary_body, encode_binary_body, BinaryChunk};
use crate::network::protocol::{HandshakeMessage, SignalMessage};

const FRAME_HANDSHAKE: u8 = 0x00;
const FRAME_DATA: u8 = 0x01;
const FRAME_BINARY: u8 = 0x02;

#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    Signal(SignalMessage),
    Binary(BinaryChunk),
}

pub enum Frame {
    Handshake(HandshakeMessage),
    Data(SignalMessage),
    Binary(BinaryChunk),
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
        FRAME_BINARY => {
            let chunk = decode_binary_body(&payload).map_err(|_| FrameError::InvalidBinaryChunk)?;
            Ok(Frame::Binary(chunk))
        }
        other => Err(FrameError::UnknownFrameType(other)),
    }
}

pub async fn write_frame(writer: &mut (impl AsyncWrite + Unpin), frame: &Frame) -> Result<(), FrameError> {
    let (frame_type, body) = match frame {
        Frame::Handshake(msg) => (FRAME_HANDSHAKE, serde_json::to_vec(msg)?),
        Frame::Data(msg) => (FRAME_DATA, serde_json::to_vec(msg)?),
        Frame::Binary(chunk) => (FRAME_BINARY, encode_binary_body(chunk)),
    };

    let len = body.len() as u32;
    let mut buf = Vec::with_capacity(5 + body.len());
    buf.push(frame_type);
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&body);

    writer.write_all(&buf).await?;
    Ok(())
}

pub async fn tcp_connect(ip: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    let stream = TcpStream::connect(format_socket_addr(ip, port)).await?;
    stream.set_nodelay(true)?;
    Ok(stream)
}

fn format_socket_addr(host: &str, port: u16) -> String {
    let trimmed = host.trim();
    if trimmed.contains(':') && !trimmed.starts_with('[') {
        format!("[{trimmed}]:{port}")
    } else {
        format!("{trimmed}:{port}")
    }
}

/// Returns true when something is accepting TCP connections on the PlanarClip port.
pub async fn probe_tcp_reachable(ip: &str, port: u16, timeout: std::time::Duration) -> bool {
    match tokio::time::timeout(timeout, TcpStream::connect(format_socket_addr(ip, port))).await {
        Ok(Ok(mut stream)) => {
            let _ = stream.shutdown().await;
            true
        }
        _ => false,
    }
}

pub async fn probe_tcp_reachable_on_any_port(
    ip: &str,
    ports: &[u16],
    timeout: std::time::Duration,
) -> Option<u16> {
    for port in ports {
        if probe_tcp_reachable(ip, *port, timeout).await {
            return Some(*port);
        }
    }
    None
}

/// Probe primary port with one retry, then fall back to alternate ports.
pub async fn probe_tcp_reachable_resilient(
    ip: &str,
    port: u16,
    alternate_ports: &[u16],
    timeout: std::time::Duration,
) -> Option<u16> {
    for attempt in 0..2 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
        if probe_tcp_reachable(ip, port, timeout).await {
            return Some(port);
        }
    }

    for &alt in alternate_ports {
        if alt == port {
            continue;
        }
        if probe_tcp_reachable(ip, alt, timeout).await {
            return Some(alt);
        }
    }

    None
}

pub fn is_likely_probe_disconnect(error: &HandshakeError) -> bool {
    match error {
        HandshakeError::Frame(FrameError::Io(io_error)) | HandshakeError::Io(io_error) => {
            is_connection_closed(io_error)
        }
        _ => false,
    }
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
    #[error("无效二进制分块")]
    InvalidBinaryChunk,
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
    #[error("对方已取消连接")]
    PeerCancelled,
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
            Self::PeerCancelled => "peer_cancelled",
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
            "rejected" => "对方拒绝了这次连接。".into(),
            "invalid_code" => "配对码不正确。".into(),
            "timeout" => "对方未及时回应，这次连接已超时。".into(),
            "cancelled" => "你已取消这次连接。".into(),
            "peer_cancelled" => "对方已取消这次连接。".into(),
            "connection_lost" => "与对方设备的连接已中断。".into(),
            "protocol_error" => "连接过程中出了点问题，请重新发起连接。".into(),
            _ => "暂时无法连接对方设备，请确认对方应用已打开，而且你们在同一局域网内。".into(),
        }
    }

    fn from_reason_code(reason: Option<&str>) -> Self {
        match reason {
            Some("invalid_code") => Self::InvalidCode,
            Some("timeout") => Self::Timeout,
            Some("cancelled") => Self::Cancelled,
            Some("peer_cancelled") => Self::PeerCancelled,
            Some("rejected") => Self::Rejected,
            Some("protocol_error") => Self::Protocol("协议状态异常"),
            _ => Self::Rejected,
        }
    }
}

pub struct DirectConnection {
    pub rx: mpsc::UnboundedReceiver<ConnectionEvent>,
    pub tx: mpsc::UnboundedSender<ConnectionEvent>,
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
    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<ConnectionEvent>();
    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<ConnectionEvent>();

    let (mut read_half, mut write_half) = stream.into_split();

    tokio::spawn(async move {
        loop {
            match read_frame(&mut read_half).await {
                Ok(Frame::Data(msg)) => {
                    if incoming_tx.send(ConnectionEvent::Signal(msg)).is_err() {
                        break;
                    }
                }
                Ok(Frame::Binary(chunk)) => {
                    if incoming_tx.send(ConnectionEvent::Binary(chunk)).is_err() {
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
        while let Some(event) = outgoing_rx.recv().await {
            let frame = match event {
                ConnectionEvent::Signal(msg) => Frame::Data(msg),
                ConnectionEvent::Binary(chunk) => Frame::Binary(chunk),
            };

            if write_frame(&mut write_half, &frame).await.is_err() {
                tracing::warn!("TCP write closed");
                break;
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

pub async fn initiator_send_connect_request(
    ip: &str,
    port: u16,
    device_name: &str,
    key_pair: &KeyPair,
    requires_confirmation: bool,
) -> Result<TcpStream, HandshakeError> {
    let mut stream = tcp_connect(ip, port).await?;

    let req = HandshakeMessage::ConnectRequest {
        device_name: device_name.to_string(),
        peer_id: key_pair.fingerprint(),
        public_key: hex::encode(key_pair.public_bytes()),
        requires_confirmation,
    };
    write_frame(&mut stream, &Frame::Handshake(req)).await?;

    Ok(stream)
}

pub async fn initiator_read_connect_response(
    mut stream: TcpStream,
) -> Result<InitiatorResult, HandshakeError> {
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

pub async fn initiator_abort(mut stream: TcpStream) {
    let _ = write_frame(
        &mut stream,
        &Frame::Handshake(HandshakeMessage::AuthResult {
            success: false,
            peer_name: None,
            public_key: None,
            reason: Some("peer_cancelled".into()),
        }),
    )
    .await;
    let _ = stream.shutdown().await;
}

#[cfg(test)]
pub async fn initiator_connect(
    ip: &str,
    port: u16,
    device_name: &str,
    key_pair: &KeyPair,
    requires_confirmation: bool,
) -> Result<InitiatorResult, HandshakeError> {
    let stream = initiator_send_connect_request(ip, port, device_name, key_pair, requires_confirmation).await?;
    initiator_read_connect_response(stream).await
}

pub fn pairing_code_from_public_key_bytes(pk_bytes: &[u8]) -> String {
    let fp = peer_id_from_public_key(pk_bytes);
    let numeric: String = fp.chars().filter(|c| c.is_ascii_digit()).collect();
    if numeric.len() >= 6 {
        numeric[numeric.len() - 6..].to_string()
    } else {
        format!(
            "{:06}",
            u32::from_str_radix(&fp[..6.min(fp.len())], 16).unwrap_or(0) % 1_000_000
        )
    }
}

pub enum InitiatorPendingMessage {
    None,
    PeerFailed(HandshakeError),
    PeerAccepted(DirectConnection),
}

pub async fn poll_initiator_peer_message(
    mut stream: TcpStream,
) -> Result<(InitiatorPendingMessage, Option<TcpStream>), HandshakeError> {
    match tokio::time::timeout(std::time::Duration::from_millis(50), stream.readable()).await {
        Ok(Ok(())) => match read_frame(&mut stream).await {
            Ok(Frame::Handshake(HandshakeMessage::AuthResult {
                success: true,
                peer_name,
                public_key,
                ..
            })) => {
                let name = peer_name.unwrap_or_default();
                let pk_bytes = public_key
                    .as_deref()
                    .and_then(|h| hex::decode(h).ok())
                    .unwrap_or_default();
                let pid = short_fingerprint(&pk_bytes);
                Ok((
                    InitiatorPendingMessage::PeerAccepted(spawn_data_bridge(
                        stream,
                        name,
                        pid,
                        pk_bytes,
                    )),
                    None,
                ))
            }
            Ok(Frame::Handshake(HandshakeMessage::AuthResult {
                success: false,
                reason,
                ..
            })) => {
                let _ = stream.shutdown().await;
                Ok((
                    InitiatorPendingMessage::PeerFailed(HandshakeError::from_reason_code(
                        reason.as_deref(),
                    )),
                    None,
                ))
            }
            Ok(_) => {
                let _ = stream.shutdown().await;
                Err(HandshakeError::Protocol(
                    "等待配对码时收到异常握手消息",
                ))
            }
            Err(e) => {
                let _ = stream.shutdown().await;
                Err(frame_error_to_handshake(e))
            }
        },
        Ok(Err(error)) => {
            let _ = stream.shutdown().await;
            Err(HandshakeError::Io(error))
        }
        Err(_) => Ok((InitiatorPendingMessage::None, Some(stream))),
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

pub async fn initiator_publish_local_code(
    stream: &mut TcpStream,
    code: String,
) -> Result<(), HandshakeError> {
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(HandshakeError::Protocol("本机配对码格式无效"));
    }

    write_frame(
        stream,
        &Frame::Handshake(HandshakeMessage::LocalPairingCode { code }),
    )
    .await
    .map_err(frame_error_to_handshake)
}

pub struct IncomingRequest {
    pub stream: TcpStream,
    pub initiator_name: String,
    pub initiator_peer_id: String,
    pub initiator_public_key: Vec<u8>,
    pub requires_confirmation: bool,
}

pub async fn read_connect_request(mut stream: TcpStream) -> Result<IncomingRequest, HandshakeError> {
    let frame = read_frame(&mut stream).await?;
    match frame {
        Frame::Handshake(HandshakeMessage::ConnectRequest {
            device_name,
            peer_id,
            public_key,
            requires_confirmation,
        }) => {
            let pk_bytes = hex::decode(&public_key)
                .map_err(|_| HandshakeError::Protocol("公钥十六进制格式无效"))?;
            Ok(IncomingRequest {
                stream,
                initiator_name: device_name,
                initiator_peer_id: peer_id,
                initiator_public_key: pk_bytes,
                requires_confirmation,
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
    pairing_code: std::sync::Arc<tokio::sync::Mutex<String>>,
    mut on_code_rotated: impl FnMut(String) + Send,
    accept_rx: oneshot::Receiver<()>,
    mut reject_rx: mpsc::Receiver<()>,
    mut timeout_rx: mpsc::Receiver<()>,
    mut submit_rx: mpsc::Receiver<(String, oneshot::Sender<Result<(), HandshakeError>>)>,
) -> Result<DirectConnection, HandshakeError> {
    // 陌生设备需先在本机确认，再通知对方进入配对码输入流程。
    wait_for_user_or_peer_abort(
        &mut stream,
        accept_rx,
        &mut reject_rx,
        &mut timeout_rx,
        PAIRING_CODE_WAIT_SECS,
    )
    .await?;

    write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AwaitCode)).await?;

    let expected_initiator_code = std::sync::Arc::new(tokio::sync::Mutex::new(
        pairing_code_from_public_key_bytes(&initiator_public_key),
    ));

    loop {
        tokio::select! {
            frame = read_frame(&mut stream) => {
                match frame {
                    Ok(Frame::Handshake(HandshakeMessage::LocalPairingCode { code })) => {
                        if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
                            *expected_initiator_code.lock().await = code;
                        }
                    }
                    Ok(Frame::Handshake(HandshakeMessage::AuthCode { code })) => {
                        if !verify_and_complete_responder_pairing(
                            &mut stream,
                            device_name,
                            key_pair,
                            &initiator_name,
                            &initiator_public_key,
                            &pairing_code,
                            code,
                        )
                        .await?
                        {
                            return Err(HandshakeError::InvalidCode);
                        }
                        let pid = short_fingerprint(&initiator_public_key);
                        return Ok(spawn_data_bridge(
                            stream,
                            initiator_name,
                            pid,
                            initiator_public_key,
                        ));
                    }
                    Ok(Frame::Handshake(HandshakeMessage::AuthResult { success: false, .. })) => {
                        return Err(HandshakeError::PeerCancelled);
                    }
                    Ok(_) => return Err(HandshakeError::Protocol("应收到配对码消息")),
                    Err(e) => return Err(frame_error_to_handshake(e)),
                }
            }
            submit = submit_rx.recv() => {
                let Some((code, reply)) = submit else {
                    return Err(HandshakeError::Cancelled);
                };
                let local_code = pairing_code.lock().await.clone();
                if code == local_code {
                    let _ = reply.send(Err(HandshakeError::InvalidCode));
                    continue;
                }
                if code != *expected_initiator_code.lock().await {
                    let _ = reply.send(Err(HandshakeError::InvalidCode));
                    continue;
                }
                let _ = reply.send(Ok(()));
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
                return Ok(spawn_data_bridge(
                    stream,
                    initiator_name,
                    pid,
                    initiator_public_key,
                ));
            }
            reject = reject_rx.recv() => {
                if reject.is_none() {
                    return Err(HandshakeError::Cancelled);
                }
                let _ = write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AuthResult {
                    success: false,
                    peer_name: None,
                    public_key: None,
                    reason: Some("peer_cancelled".into()),
                })).await;
                return Err(HandshakeError::Cancelled);
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(PAIRING_CODE_WAIT_SECS)) => {
                let new_code = generate_pairing_code();
                {
                    let mut guard = pairing_code.lock().await;
                    *guard = new_code.clone();
                }
                on_code_rotated(new_code);
            }
        }
    }
}

async fn verify_and_complete_responder_pairing(
    stream: &mut TcpStream,
    device_name: &str,
    key_pair: &KeyPair,
    initiator_name: &str,
    initiator_public_key: &[u8],
    pairing_code: &std::sync::Arc<tokio::sync::Mutex<String>>,
    received_code: String,
) -> Result<bool, HandshakeError> {
    let expected_code = pairing_code.lock().await.clone();
    if received_code != expected_code {
        write_frame(
            stream,
            &Frame::Handshake(HandshakeMessage::AuthResult {
                success: false,
                peer_name: None,
                public_key: None,
                reason: Some("invalid_code".into()),
            }),
        )
        .await?;
        return Ok(false);
    }

    let pk_hex = hex::encode(key_pair.public_bytes());
    write_frame(
        stream,
        &Frame::Handshake(HandshakeMessage::AuthResult {
            success: true,
            peer_name: Some(device_name.to_string()),
            public_key: Some(pk_hex),
            reason: None,
        }),
    )
    .await?;

    let _ = initiator_name;
    let _ = initiator_public_key;
    Ok(true)
}

fn is_connection_closed(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::UnexpectedEof
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::BrokenPipe
    )
}

fn frame_error_to_handshake(error: FrameError) -> HandshakeError {
    if let FrameError::Io(ref io_error) = error {
        if is_connection_closed(io_error) {
            return HandshakeError::PeerCancelled;
        }
    }
    HandshakeError::Frame(error)
}

async fn read_peer_abort_signal(stream: &mut TcpStream) -> Result<(), HandshakeError> {
    stream.readable().await?;
    match read_frame(stream).await {
        Ok(Frame::Handshake(HandshakeMessage::AuthResult {
            success: false,
            reason,
            ..
        })) => Err(HandshakeError::from_reason_code(reason.as_deref())),
        Ok(_) => Err(HandshakeError::Protocol("等待连接确认时收到异常握手消息")),
        Err(e) => Err(frame_error_to_handshake(e)),
    }
}

async fn notify_peer_response_timeout(stream: &mut TcpStream) {
    let _ = write_frame(
        stream,
        &Frame::Handshake(HandshakeMessage::AuthResult {
            success: false,
            peer_name: None,
            public_key: None,
            reason: Some("timeout".into()),
        }),
    )
    .await;
}

async fn wait_for_user_or_peer_abort(
    stream: &mut TcpStream,
    accept_rx: oneshot::Receiver<()>,
    reject_rx: &mut mpsc::Receiver<()>,
    timeout_rx: &mut mpsc::Receiver<()>,
    timeout_secs: u64,
) -> Result<(), HandshakeError> {
    let mut accept_rx = accept_rx;
    loop {
        tokio::select! {
            biased;
            abort = read_peer_abort_signal(stream) => return abort,
            _ = &mut accept_rx => return Ok(()),
            reject = reject_rx.recv() => {
                if reject.is_none() {
                    return Err(HandshakeError::Cancelled);
                }
                let _ = write_frame(stream, &Frame::Handshake(HandshakeMessage::AuthResult {
                    success: false,
                    peer_name: None,
                    public_key: None,
                    reason: Some("rejected".into()),
                })).await;
                return Err(HandshakeError::Cancelled);
            }
            _ = timeout_rx.recv() => {
                notify_peer_response_timeout(stream).await;
                return Err(HandshakeError::Timeout);
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                notify_peer_response_timeout(stream).await;
                return Err(HandshakeError::Timeout);
            }
        }
    }
}

pub async fn responder_wait_for_decision(
    mut stream: TcpStream,
    device_name: &str,
    key_pair: &KeyPair,
    initiator_name: String,
    initiator_public_key: Vec<u8>,
    accept_rx: oneshot::Receiver<()>,
    mut reject_rx: mpsc::Receiver<()>,
    mut timeout_rx: mpsc::Receiver<()>,
) -> Result<DirectConnection, HandshakeError> {
    wait_for_user_or_peer_abort(&mut stream, accept_rx, &mut reject_rx, &mut timeout_rx, 60).await?;

    responder_accept_trusted(
        stream,
        device_name,
        key_pair,
        initiator_name,
        initiator_public_key,
    )
    .await
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
                    if is_likely_probe_disconnect(&e) {
                        tracing::debug!(
                            "Incomplete TCP handshake from {} (likely reachability probe): {}",
                            addr,
                            e
                        );
                    } else {
                        tracing::warn!("Failed to read ConnectRequest from {}: {}", addr, e);
                    }
                }
            }
        });
    }
}

pub(crate) fn short_fingerprint(pk_bytes: &[u8]) -> String {
    peer_id_from_public_key(pk_bytes)
}

pub const PAIRING_CODE_WAIT_SECS: u64 = 60;

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

    fn idle_submit_channel(
    ) -> (
        mpsc::Sender<(String, oneshot::Sender<Result<(), HandshakeError>>)>,
        mpsc::Receiver<(String, oneshot::Sender<Result<(), HandshakeError>>)>,
    ) {
        mpsc::channel(4)
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

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key, false)
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
            let (accept_tx, accept_rx) = oneshot::channel();
            let (_reject_tx, reject_rx) = mpsc::channel(2);
            let (_timeout_tx, timeout_rx) = mpsc::channel(1);
            let pairing_code = std::sync::Arc::new(tokio::sync::Mutex::new("123456".to_string()));
            let (_submit_tx, submit_rx) = idle_submit_channel();
            let verify = tokio::spawn(async move {
                responder_verify_code(
                    request.stream,
                    "Responder",
                    &responder_key_for_server,
                    request.initiator_name,
                    request.initiator_public_key,
                    pairing_code,
                    |_| {},
                    accept_rx,
                    reject_rx,
                    timeout_rx,
                    submit_rx,
                )
                .await
            });
            accept_tx.send(()).unwrap();
            verify.await.unwrap().unwrap()
        });

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key, false)
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
    async fn confirmed_unknown_peer_connects_without_code() {
        let (listener, port) = bind_test_listener().await;
        let initiator_key = KeyPair::generate();
        let responder_key = KeyPair::generate();
        let responder_key_for_server = responder_key.clone();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let request = read_connect_request(stream).await.unwrap();
            let (accept_tx, accept_rx) = oneshot::channel();
            let (_reject_tx, reject_rx) = mpsc::channel(2);
            let (_timeout_tx, timeout_rx) = mpsc::channel(1);
            let decision = tokio::spawn(async move {
                responder_wait_for_decision(
                    request.stream,
                    "Responder",
                    &responder_key_for_server,
                    request.initiator_name,
                    request.initiator_public_key,
                    accept_rx,
                    reject_rx,
                    timeout_rx,
                )
                .await
            });
            accept_tx.send(()).unwrap();
            decision.await.unwrap().unwrap()
        });

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key, false)
            .await
            .unwrap();
        let connection = match result {
            InitiatorResult::Connected(connection) => connection,
            InitiatorResult::AwaitingCode { .. } => panic!("confirmed peer should connect directly"),
        };

        assert_eq!(connection.peer_name, "Responder");
        let server_connection = server.await.unwrap();
        assert_eq!(server_connection.peer_name, "Initiator");
    }

    #[tokio::test]
    async fn initiator_confirmation_flag_blocks_responder_auto_accept() {
        let (listener, port) = bind_test_listener().await;
        let initiator_key = KeyPair::generate();
        let responder_key = KeyPair::generate();
        let initiator_key_for_server = initiator_key.clone();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let request = read_connect_request(stream).await.unwrap();
            assert!(request.requires_confirmation);
            let (accept_tx, accept_rx) = oneshot::channel();
            let (_reject_tx, reject_rx) = mpsc::channel(2);
            let (_timeout_tx, timeout_rx) = mpsc::channel(1);
            let decision = tokio::spawn(async move {
                responder_wait_for_decision(
                    request.stream,
                    "Responder",
                    &responder_key,
                    request.initiator_name,
                    request.initiator_public_key,
                    accept_rx,
                    reject_rx,
                    timeout_rx,
                )
                .await
            });
            accept_tx.send(()).unwrap();
            decision.await.unwrap().unwrap()
        });

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key_for_server, true)
            .await
            .unwrap();
        let connection = match result {
            InitiatorResult::Connected(connection) => connection,
            InitiatorResult::AwaitingCode { .. } => panic!("confirmation path should connect after accept"),
        };

        assert_eq!(connection.peer_name, "Responder");
        let _ = server.await.unwrap();
    }

    #[tokio::test]
    async fn pairing_requires_confirmation_before_await_code() {
        let (listener, port) = bind_test_listener().await;
        let initiator_key = KeyPair::generate();
        let responder_key = KeyPair::generate();
        let responder_key_for_server = responder_key.clone();

        let (accept_tx, accept_rx) = oneshot::channel();
        let (_reject_tx, reject_rx) = mpsc::channel(2);
        let (_timeout_tx, timeout_rx) = mpsc::channel(1);

        let pairing_code = std::sync::Arc::new(tokio::sync::Mutex::new("123456".to_string()));
        let (_submit_tx, submit_rx) = idle_submit_channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let request = read_connect_request(stream).await.unwrap();
            responder_verify_code(
                request.stream,
                "Responder",
                &responder_key_for_server,
                request.initiator_name,
                request.initiator_public_key,
                pairing_code,
                |_| {},
                accept_rx,
                reject_rx,
                timeout_rx,
                submit_rx,
            )
            .await
            .unwrap()
        });

        let initiator = tokio::spawn(async move {
            initiator_connect("127.0.0.1", port, "Initiator", &initiator_key, false).await
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(!initiator.is_finished(), "initiator should wait for confirmation");

        accept_tx.send(()).unwrap();

        let result = initiator.await.unwrap().unwrap();
        let stream = match result {
            InitiatorResult::AwaitingCode { stream } => stream,
            InitiatorResult::Connected(_) => panic!("unknown peer should require pairing code after confirmation"),
        };

        let connection = initiator_send_code(stream, "123456".into()).await.unwrap();
        assert_eq!(connection.peer_name, "Responder");
        let _ = server.await.unwrap();
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
            let (accept_tx, accept_rx) = oneshot::channel();
            let (_reject_tx, reject_rx) = mpsc::channel(2);
            let (_timeout_tx, timeout_rx) = mpsc::channel(1);
            let pairing_code = std::sync::Arc::new(tokio::sync::Mutex::new("123456".to_string()));
            let (_submit_tx, submit_rx) = idle_submit_channel();
            let verify = tokio::spawn(async move {
                responder_verify_code(
                    request.stream,
                    "Responder",
                    &responder_key_for_server,
                    request.initiator_name,
                    request.initiator_public_key,
                    pairing_code,
                    |_| {},
                    accept_rx,
                    reject_rx,
                    timeout_rx,
                    submit_rx,
                )
                .await
            });
            accept_tx.send(()).unwrap();
            verify.await.unwrap()
        });

        let result = initiator_connect("127.0.0.1", port, "Initiator", &initiator_key, false)
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
