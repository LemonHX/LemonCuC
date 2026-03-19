//! WebSocket-to-TCP proxy (websockify).
//!
//! Accepts incoming WebSocket connections and bidirectionally proxies
//! them to a target TCP address (typically a VNC server on :5900).

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

/// Run the websockify proxy.
///
/// * `listen_addr` – e.g. `"0.0.0.0:6080"`
/// * `target_addr` – e.g. `"127.0.0.1:5900"`
pub async fn run(listen_addr: &str, target_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("websockify listening on {listen_addr} → {target_addr}");

    let target = target_addr.to_owned();

    loop {
        let (stream, peer) = listener.accept().await?;
        let target = target.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, &target).await {
                tracing::warn!("connection from {peer} ended: {e}");
            }
        });
    }
}

/// Handle a single WebSocket client.
async fn handle_client(
    stream: TcpStream,
    target_addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Upgrade the incoming TCP stream to a WebSocket connection.
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    tracing::debug!("WebSocket handshake complete, connecting to {target_addr}");

    // Connect to the VNC (or other TCP) target.
    let tcp_stream = TcpStream::connect(target_addr).await?;
    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();
    let (mut ws_write, mut ws_read) = ws_stream.split();

    // WS → TCP: read binary frames from the WebSocket client and write to TCP target.
    let ws_to_tcp = async move {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if tcp_write.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                // Ignore text / ping / pong
                _ => {}
            }
        }
        let _ = tcp_write.shutdown().await;
    };

    // TCP → WS: read bytes from the TCP target and send as binary frames.
    let tcp_to_ws = async move {
        let mut buf = vec![0u8; 65536];
        loop {
            let n = match tcp_read.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let data = buf[..n].to_vec();
            if ws_write.send(Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
        let _ = ws_write.close().await;
    };

    // Run both directions concurrently; when either side closes we're done.
    tokio::select! {
        _ = ws_to_tcp => {},
        _ = tcp_to_ws => {},
    }

    Ok(())
}
