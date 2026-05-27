use tokio_tungstenite::connect_async;
use futures_util::StreamExt;
use tokio::sync::mpsc;

/// Connect to the signalling server and return a channel for incoming messages.
/// For MVP, uses a public relay (hardcoded). Future: configurable.
pub async fn connect(
    server_url: &str,
    room: &str,
    peer_id: &str,
) -> Result<mpsc::UnboundedReceiver<String>, Box<dyn std::error::Error>> {
    let url = format!("{}/room/{}/peer/{}", server_url, room, peer_id);
    let (ws_stream, _) = connect_async(&url).await?;
    let (_, mut read) = ws_stream.split();

    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            if let Ok(text) = msg.to_text() {
                let _ = tx.send(text.to_string());
            }
        }
    });

    Ok(rx)
}
