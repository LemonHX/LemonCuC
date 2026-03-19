//! POST /api/i3msg – send i3 IPC messages.

use axum::Json;
use axum_openapi3::endpoint;
use axum_openapi3::utoipa;
use axum_openapi3::utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Request body for i3-msg.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct I3MsgRequest {
    /// i3-msg message type, e.g. `"command"`, `"get_workspaces"`, `"get_tree"`.
    #[serde(default = "default_msg_type")]
    pub msg_type: String,
    /// The payload/command string, e.g. `"workspace 2"` or `"exec chromium"`.
    pub payload: String,
}

fn default_msg_type() -> String {
    "command".into()
}

/// Response from i3-msg.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct I3MsgResponse {
    pub success: bool,
    /// Raw JSON output from i3-msg.
    pub output: String,
}

#[endpoint(
    method = "POST",
    path = "/api/i3msg",
    description = "Send i3 IPC messages"
)]
pub async fn api_i3msg(Json(req): Json<I3MsgRequest>) -> Json<I3MsgResponse> {
    let result = Command::new("i3-msg")
        .args(["-t", &req.msg_type, &req.payload])
        .output()
        .await;

    match result {
        Ok(output) => Json(I3MsgResponse {
            success: output.status.success(),
            output: String::from_utf8_lossy(&output.stdout).into_owned(),
        }),
        Err(e) => Json(I3MsgResponse {
            success: false,
            output: e.to_string(),
        }),
    }
}
