use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use crate::crypto::keys::KeyPair;
use crate::network::protocol::{HandshakeMessage, SignalMessage};

// ── Frame codec ──────────────────────────────────────────────────────────

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

// ── Error types ──────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Payload too large: {0} bytes")]
    PayloadTooLarge(usize),
    #[error("Unknown frame type: 0x{0:02x}")]
    UnknownFrameType(u8),
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Frame error: {0}")]
    Frame(#[from] FrameError),
    #[error("Connection rejected by peer")]
    Rejected,
    #[error("Invalid pairing code")]
    InvalidCode,
    #[error("Pairing code expired")]
    Timeout,
    #[error("Cancelled by user")]
    Cancelled,
    #[error("Protocol error: {0}")]
    Protocol(&'static str),
}

// ── DirectConnection — the post-handshake transport ──────────────────────

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

    // Task: read Data frames from TCP → push to incoming channel
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

    // Task: read from outgoing channel → write Data frames to TCP
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

// ── Initiator handshake ──────────────────────────────────────────────────

pub enum InitiatorResult {
    Connected(DirectConnection),
    AwaitingCode { stream: TcpStream },
}

/// Run the initiator side of the handshake up to the point where a decision
/// is needed (auto-accept vs. pairing code required).
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
        }) => {
            let name = peer_name.unwrap_or_default();
            let pk_bytes = public_key
                .as_deref()
                .and_then(|h| hex::decode(h).ok())
                .unwrap_or_default();
            let pid = short_fingerprint(&pk_bytes);
            Ok(InitiatorResult::Connected(spawn_data_bridge(
                stream, name, pid, pk_bytes,
            )))
        }
        Frame::Handshake(HandshakeMessage::AwaitCode) => {
            Ok(InitiatorResult::AwaitingCode { stream })
        }
        Frame::Handshake(HandshakeMessage::AuthResult { success: false, .. }) => {
            Err(HandshakeError::Rejected)
        }
        _ => Err(HandshakeError::Protocol(
            "expected AuthResult or AwaitCode after ConnectRequest",
        )),
    }
}

/// Send the pairing code and complete the initiator handshake.
///
/// Returns the established connection and the responder's peer info (for
/// saving to trusted_peers).
pub async fn initiator_send_code(
    mut stream: TcpStream,
    code: String,
) -> Result<DirectConnection, HandshakeError> {
    write_frame(
        &mut stream,
        &Frame::Handshake(HandshakeMessage::AuthCode { code }),
    )
    .await?;

    match read_frame(&mut stream).await? {
        Frame::Handshake(HandshakeMessage::AuthResult {
            success: true,
            peer_name,
            public_key,
        }) => {
            let name = peer_name.unwrap_or_default();
            let pk_bytes = public_key
                .as_deref()
                .and_then(|h| hex::decode(h).ok())
                .unwrap_or_default();
            let pid = short_fingerprint(&pk_bytes);
            Ok(spawn_data_bridge(stream, name, pid, pk_bytes))
        }
        Frame::Handshake(HandshakeMessage::AuthResult { success: false, .. }) => {
            Err(HandshakeError::InvalidCode)
        }
        _ => Err(HandshakeError::Protocol(
            "expected AuthResult after AuthCode",
        )),
    }
}

// ── Responder handshake ──────────────────────────────────────────────────

/// Info extracted from an incoming ConnectRequest.
pub struct IncomingRequest {
    pub stream: TcpStream,
    pub initiator_name: String,
    pub initiator_peer_id: String,
    pub initiator_public_key: Vec<u8>,
}

/// Read a ConnectRequest from an accepted TCP stream.
pub async fn read_connect_request(mut stream: TcpStream) -> Result<IncomingRequest, HandshakeError> {
    let frame = read_frame(&mut stream).await?;
    match frame {
        Frame::Handshake(HandshakeMessage::ConnectRequest {
            device_name,
            peer_id,
            public_key,
        }) => {
            let pk_bytes = hex::decode(&public_key)
                .map_err(|_| HandshakeError::Protocol("invalid public key hex"))?;
            Ok(IncomingRequest {
                stream,
                initiator_name: device_name,
                initiator_peer_id: peer_id,
                initiator_public_key: pk_bytes,
            })
        }
        _ => Err(HandshakeError::Protocol("expected ConnectRequest")),
    }
}

/// Complete the responder handshake for a trusted peer (auto-accept).
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
        }),
    )
    .await?;

    let pid = short_fingerprint(&initiator_public_key);
    Ok(spawn_data_bridge(
        stream,
        initiator_name,
        pid,
        initiator_public_key,
    ))
}

/// Complete the responder handshake for an unknown peer (pairing code flow).
///
/// Sends `AwaitCode`, then waits for the AuthCode, a reject signal, or timeout.
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
                _ => return Err(HandshakeError::Protocol("expected AuthCode")),
            }
        }
        _ = reject_rx => {
            let _ = write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AuthResult {
                success: false, peer_name: None, public_key: None,
            })).await;
            return Err(HandshakeError::Cancelled);
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
            let _ = write_frame(&mut stream, &Frame::Handshake(HandshakeMessage::AuthResult {
                success: false, peer_name: None, public_key: None,
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
        }),
    )
    .await?;

    let pid = short_fingerprint(&initiator_public_key);
    Ok(spawn_data_bridge(
        stream,
        initiator_name,
        pid,
        initiator_public_key,
    ))
}

// ── TCP listener ─────────────────────────────────────────────────────────

/// Events emitted by the TCP listener to the coordinator (lib.rs).
pub enum ListenerEvent {
    /// A new incoming connection with its ConnectRequest already read.
    Incoming(IncomingRequest),
}

/// Bind a TCP listener and push each accepted connection as a `ListenerEvent`.
///
/// The caller (lib.rs) receives events and decides how to handle each request
/// (auto-accept vs. pairing code flow).
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

// ── Helpers ──────────────────────────────────────────────────────────────

pub(crate) fn short_fingerprint(pk_bytes: &[u8]) -> String {
    let hex = hex::encode(pk_bytes);
    hex[..6].to_string()
}

pub(crate) fn generate_pairing_code() -> String {
    use rand::Rng;
    format!("{:06}", rand::thread_rng().gen_range(0..1_000_000))
}
