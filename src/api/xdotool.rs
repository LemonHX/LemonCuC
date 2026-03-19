//! POST /api/xdotool – run xdotool commands for input simulation.

use axum::Json;
use axum_openapi3::endpoint;
use axum_openapi3::utoipa;
use axum_openapi3::utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Request body for xdotool.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct XdotoolRequest {
    /// xdotool sub-command and arguments, e.g. `["type", "--clearmodifiers", "hello"]`
    pub args: Vec<String>,
}

/// Response from xdotool.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct XdotoolResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[endpoint(
    method = "POST",
    path = "/api/xdotool",
    description = "Run xdotool commands for input simulation"
)]
pub async fn api_xdotool(Json(req): Json<XdotoolRequest>) -> Json<XdotoolResponse> {
    let result = Command::new("xdotool").args(&req.args).output().await;

    match result {
        Ok(output) => Json(XdotoolResponse {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
        Err(e) => Json(XdotoolResponse {
            success: false,
            stdout: String::new(),
            stderr: e.to_string(),
        }),
    }
}
