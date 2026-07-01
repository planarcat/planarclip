use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::network::protocol::SignalMessage;

/// Bidirectional signalling client connected to the WebSocket relay.
pub struct SignallingClient {
    /// Incoming parsed messages from the relay.
    pub rx: mpsc::UnboundedReceiver<SignalMessage>,
    /// Outgoing channel sender used by the connection layer to queue messages.
    pub(crate) tx: mpsc::UnboundedSender<String>,
}

/// Connect to the signalling relay at `{server_url}/room/{room}/peer/{peer_id}`.
pub async fn connect(
    server_url: &str,
    room: &str,
    peer_id: &str,
) -> Result<SignallingClient, Box<dyn std::error::Error>> {
    let url = format!("{}/room/{}/peer/{}", server_url, room, peer_id);
    tracing::info!("Connecting to signalling server: {}", url);

    let (ws_stream, _) = connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<String>();
    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<SignalMessage>();

    tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<SignalMessage>(&text) {
                    Ok(signal_msg) => {
                        let _ = incoming_tx.send(signal_msg);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse signalling message: {} — raw: {}", e, text);
                    }
                },
                Message::Close(_) => {
                    tracing::info!("Signalling WebSocket closed");
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::spawn(async move {
        while let Some(text) = outgoing_rx.recv().await {
            if write.send(Message::Text(text.into())).await.is_err() {
                tracing::error!("Failed to write to signalling WebSocket");
                break;
            }
        }
    });

    Ok(SignallingClient {
        rx: incoming_rx,
        tx: outgoing_tx,
    })
}
