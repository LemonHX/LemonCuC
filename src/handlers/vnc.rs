//! /ws/vnc – WebSocket-to-TCP proxy for VNC (x11vnc on :5900).

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const VNC_TARGET: &str = "127.0.0.1:5900";

pub async fn ws_vnc(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_vnc)
}

async fn handle_vnc(ws: WebSocket) {
    if let Err(e) = proxy(ws, VNC_TARGET).await {
        tracing::warn!("vnc ws proxy ended: {e}");
    }
}

async fn proxy(
    ws: WebSocket,
    target: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tcp = TcpStream::connect(target).await?;
    let (mut tcp_read, mut tcp_write) = tcp.into_split();
    let (mut ws_write, mut ws_read) = ws.split();

    let ws_to_tcp = async move {
        while let Some(Ok(msg)) = ws_read.next().await {
            match msg {
                Message::Binary(data) => {
                    if tcp_write.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        let _ = tcp_write.shutdown().await;
    };

    let tcp_to_ws = async move {
        let mut buf = vec![0u8; 65536];
        loop {
            let n = match tcp_read.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            if ws_write
                .send(Message::Binary(buf[..n].to_vec().into()))
                .await
                .is_err()
            {
                break;
            }
        }
        let _ = ws_write.close().await;
    };

    tokio::select! {
        _ = ws_to_tcp => {},
        _ = tcp_to_ws => {},
    }

    Ok(())
}
