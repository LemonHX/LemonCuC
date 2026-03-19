//! /ws/ssh – WebSocket ↔ local SSH (PTY mode, multi-session).
//!
//! Each WS connection spawns a new `ssh root@127.0.0.1 -tt` process.
//! The Dockerfile pre-generates an ed25519 key pair for passwordless auth.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

pub async fn ws_ssh(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_ssh)
}

async fn handle_ssh(ws: WebSocket) {
    if let Err(e) = ssh_session(ws).await {
        tracing::warn!("ssh ws ended: {e}");
    }
}

async fn ssh_session(ws: WebSocket) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut child = Command::new("ssh")
        .env("TERM", "xterm-256color")
        .args([
            "-tt",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-o",
            "LogLevel=ERROR",
            "-o",
            "SendEnv=TERM",
            "root@127.0.0.1",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let (mut ws_write, mut ws_read) = ws.split();

    // WS → SSH stdin
    let ws_to_ssh = async move {
        while let Some(Ok(msg)) = ws_read.next().await {
            match msg {
                Message::Binary(data) => {
                    if stdin.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Message::Text(text) => {
                    if stdin.write_all(text.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        let _ = stdin.shutdown().await;
    };

    // SSH stdout → WS
    let ssh_to_ws = async move {
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

    tokio::select! {
        _ = ws_to_ssh => {},
        _ = ssh_to_ws => {},
    }

    let _ = child.kill().await;
    Ok(())
}
