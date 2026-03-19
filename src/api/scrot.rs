//! GET /api/scrot – capture a screenshot and return it as base64-encoded PNG.

use axum::Json;
use axum::http::StatusCode;
use axum_openapi3::endpoint;
use axum_openapi3::utoipa;
use axum_openapi3::utoipa::ToSchema;
use base64::Engine;
use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Serialize, ToSchema)]
pub struct ScrotResponse {
    /// Base64-encoded PNG screenshot (standard base64, no padding).
    pub image: String,
}

#[endpoint(
    method = "GET",
    path = "/api/scrot",
    description = "Capture a screenshot and return base64-encoded PNG"
)]
pub async fn api_scrot() -> Result<Json<ScrotResponse>, StatusCode> {
    let path = "/tmp/lemoncuc_scrot.png";

    let status = Command::new("scrot")
        .args(["-o", path])
        .status()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !status.success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let data = tokio::fs::read(path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = tokio::fs::remove_file(path).await;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    Ok(Json(ScrotResponse { image: b64 }))
}
