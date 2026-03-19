//! POST /api/dmenu – run dmenu (or rofi) for selection UI.

use axum::Json;
use axum_openapi3::endpoint;
use axum_openapi3::utoipa;
use axum_openapi3::utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Request body for dmenu.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DmenuRequest {
    /// Items to display in dmenu (one per line, piped to stdin).
    pub items: Vec<String>,
    /// Optional prompt string (`-p` flag).
    #[serde(default)]
    pub prompt: Option<String>,
}

/// Response from dmenu.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DmenuResponse {
    pub success: bool,
    /// The selected item (trimmed), or empty if cancelled.
    pub selected: String,
}

#[endpoint(
    method = "POST",
    path = "/api/dmenu",
    description = "Run dmenu for selection UI"
)]
pub async fn api_dmenu(Json(req): Json<DmenuRequest>) -> Json<DmenuResponse> {
    let mut cmd = Command::new("dmenu");
    if let Some(ref prompt) = req.prompt {
        cmd.args(["-p", prompt]);
    }
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let child = cmd.spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return Json(DmenuResponse {
                success: false,
                selected: e.to_string(),
            });
        }
    };

    // Write items to dmenu's stdin
    if let Some(mut stdin) = child.stdin.take() {
        let input = req.items.join("\n");
        let _ = stdin.write_all(input.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    match child.wait_with_output().await {
        Ok(output) => Json(DmenuResponse {
            success: output.status.success(),
            selected: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        }),
        Err(e) => Json(DmenuResponse {
            success: false,
            selected: e.to_string(),
        }),
    }
}
