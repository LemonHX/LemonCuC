//! /ws/audio – WebSocket audio streaming via GStreamer (tcpulse).

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

pub async fn ws_audio(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_audio)
}

async fn handle_audio(ws: WebSocket) {
    if let Err(e) = stream_audio(ws).await {
        tracing::warn!("audio ws ended: {e}");
    }
}

async fn stream_audio(ws: WebSocket) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(crate::tcpulse::DEFAULT_COMMAND)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let (mut ws_write, mut ws_read) = ws.split();

    let gst_to_ws = async move {
        let mut buf = vec![0u8; 4096];
        loop {
            let n = match stdout.read(&mut buf).await {
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

    // Discard anything the client sends; close when they disconnect.
    let ws_drain = async move {
        while let Some(Ok(msg)) = ws_read.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    };

    tokio::select! {
        _ = gst_to_ws => {},
        _ = ws_drain => {},
    }

    let _ = child.kill().await;
    Ok(())
}
