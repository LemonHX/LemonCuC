//! /ws/cdp – Chrome DevTools Protocol WebSocket proxy.
//!
//! On WS connect: ensure Chrome is running → discover browser WS endpoint
//! via /json/version → accept client upgrade → connect to Chrome CDP →
//! proxy frames bidirectionally.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;

const CDP_HOST: &str = "127.0.0.1";
const CDP_PORT: u16 = 9222;

pub async fn ws_cdp(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_cdp)
}

async fn handle_cdp(ws: WebSocket) {
    if let Err(e) = cdp_proxy(ws).await {
        tracing::warn!("cdp ws proxy ended: {e}");
    }
}

// ── Chrome lifecycle ────────────────────────────────────────────────────────

async fn cdp_is_up() -> bool {
    TcpStream::connect((CDP_HOST, CDP_PORT)).await.is_ok()
}

async fn ensure_chrome() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if cdp_is_up().await {
        return Ok(());
    }

    tracing::info!("Chrome not running – launching google-chrome-stable");
    let mut child = tokio::process::Command::new("google-chrome-stable")
        .arg("--remote-debugging-port=9222")
        .arg("--user-data-dir=/root/cdp_data")
        .arg("about:blank")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(target: "chrome", "{line}");
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(target: "chrome", "{line}");
            }
        });
    }

    for _ in 0..75 {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if cdp_is_up().await {
            tracing::info!("CDP is now available on :{CDP_PORT}");
            return Ok(());
        }
    }

    Err("timeout waiting for Chrome CDP to start".into())
}

// ── Discover browser WS URL ─────────────────────────────────────────────────

/// GET /json/version → extract webSocketDebuggerUrl
async fn discover_browser_ws_url() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let v: serde_json::Value = reqwest::get(format!("http://{CDP_HOST}:{CDP_PORT}/json/version"))
        .await?
        .json()
        .await?;
    let url = v["webSocketDebuggerUrl"]
        .as_str()
        .ok_or("webSocketDebuggerUrl not found")?
        .to_string();

    tracing::info!("CDP browser WS URL: {url}");
    Ok(url)
}

// ── WS proxy ────────────────────────────────────────────────────────────────

async fn cdp_proxy(client_ws: WebSocket) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_chrome().await?;

    let ws_url = discover_browser_ws_url().await?;

    let (chrome_ws, _) = tokio_tungstenite::connect_async(&ws_url).await?;
    tracing::info!("CDP WS: connected to Chrome, proxying");

    let (mut chrome_write, mut chrome_read) = chrome_ws.split();
    let (mut client_write, mut client_read) = client_ws.split();

    use tokio_tungstenite::tungstenite::Message as TMsg;

    let client_to_chrome = async move {
        while let Some(Ok(msg)) = client_read.next().await {
            let tmsg = match msg {
                Message::Text(t) => TMsg::text(t.to_string()),
                Message::Binary(b) => TMsg::binary(b.to_vec()),
                Message::Close(_) => break,
                Message::Ping(d) => TMsg::Ping(d.to_vec().into()),
                Message::Pong(d) => TMsg::Pong(d.to_vec().into()),
            };
            if chrome_write.send(tmsg).await.is_err() {
                break;
            }
        }
        let _ = chrome_write.close().await;
    };

    let chrome_to_client = async move {
        while let Some(Ok(msg)) = chrome_read.next().await {
            let amsg = match msg {
                TMsg::Text(t) => Message::Text(t.to_string().into()),
                TMsg::Binary(b) => Message::Binary(b.to_vec().into()),
                TMsg::Close(_) => break,
                TMsg::Ping(d) => Message::Ping(d.to_vec().into()),
                TMsg::Pong(d) => Message::Pong(d.to_vec().into()),
                _ => continue,
            };
            if client_write.send(amsg).await.is_err() {
                break;
            }
        }
        let _ = client_write.close().await;
    };

    tokio::select! {
        _ = client_to_chrome => {},
        _ = chrome_to_client => {},
    }

    Ok(())
}
