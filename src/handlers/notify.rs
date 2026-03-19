//! /ws/notify – push desktop notifications to the browser via WebSocket.
//!
//! We monitor D-Bus `org.freedesktop.Notifications` by spawning
//! `dbus-monitor` and parsing its output for Notify method calls.
//! A proper zbus implementation can replace this later.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

pub async fn ws_notify(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_notify)
}

async fn handle_notify(ws: WebSocket) {
    if let Err(e) = notify_stream(ws).await {
        tracing::warn!("notify ws ended: {e}");
    }
}

async fn notify_stream(ws: WebSocket) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Monitor D-Bus for notification calls
    let mut child = Command::new("dbus-monitor")
        .args([
            "--session",
            "interface='org.freedesktop.Notifications',member='Notify'",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout).lines();
    let (mut ws_write, mut ws_read) = ws.split();

    // We'll accumulate lines from dbus-monitor and emit JSON notifications.
    // dbus-monitor output for a Notify call looks roughly like:
    //   method call ... member=Notify
    //   string "app_name"
    //   ...
    //   string "summary"
    //   string "body"
    // We do simple parsing here; can be replaced by zbus later.

    let monitor_to_ws = async move {
        let mut in_notify = false;
        let mut strings: Vec<String> = Vec::new();

        while let Ok(Some(line)) = reader.next_line().await {
            if line.contains("member=Notify") {
                in_notify = true;
                strings.clear();
                continue;
            }

            if in_notify {
                // Extract string values
                if let Some(start) = line.find("string \"") {
                    let rest = &line[start + 8..];
                    if let Some(end) = rest.rfind('"') {
                        strings.push(rest[..end].to_string());
                    }
                }

                // Notify has: app_name, replaces_id, icon, summary, body, ...
                // We emit after collecting at least 5 strings (summary is index 3, body is 4)
                if strings.len() >= 5 {
                    let json = serde_json::json!({
                        "type": "notification",
                        "app_name": strings[0],
                        "icon": strings[2],
                        "summary": strings[3],
                        "body": strings[4],
                    });
                    if ws_write
                        .send(Message::Text(json.to_string().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    in_notify = false;
                    strings.clear();
                }
            }
        }
        let _ = ws_write.close().await;
    };

    let ws_drain = async move {
        while let Some(Ok(msg)) = ws_read.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    };

    tokio::select! {
        _ = monitor_to_ws => {},
        _ = ws_drain => {},
    }

    let _ = child.kill().await;
    Ok(())
}
