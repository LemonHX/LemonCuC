//! LemonCUC backend – single-port axum server with WS + REST API.

mod api;
mod handlers;
mod tcpulse;

use axum::Json;
use axum::response::IntoResponse;
use axum::routing::get;
use axum_openapi3::utoipa::openapi::{InfoBuilder, OpenApiBuilder};
use axum_openapi3::{AddRoute, build_openapi, endpoint};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    tracing::info!("LemonCUC backend starting");

    let app = axum::Router::new()
        // ── WebSocket endpoints (not in OpenAPI) ──
        .route("/ws/vnc", get(handlers::vnc::ws_vnc))
        .route("/ws/audio", get(handlers::audio::ws_audio))
        .route("/ws/ssh", get(handlers::ssh::ws_ssh))
        .route("/ws/notify", get(handlers::notify::ws_notify))
        .route("/ws/cdp", get(handlers::cdp::ws_cdp))
        // ── REST API (with OpenAPI docs) ──
        .add(api::xdotool::api_xdotool())
        .add(api::dmenu::api_dmenu())
        .add(api::i3msg::api_i3msg())
        .add(api::properties::api_get_properties())
        .add(api::scrot::api_scrot())
        // ── OpenAPI JSON endpoint ──
        .add(serve_openapi())
        // ── CORS ──
        .layer(CorsLayer::permissive());

    // Bind and serve
    let listener = tokio::net::TcpListener::bind("0.0.0.0:6080").await?;
    tracing::info!("listening on 0.0.0.0:6080");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve the generated OpenAPI JSON document.
#[endpoint(
    method = "GET",
    path = "/api/openapi.json",
    description = "OpenAPI specification"
)]
async fn serve_openapi() -> impl IntoResponse {
    let openapi = build_openapi(|| {
        OpenApiBuilder::new().info(
            InfoBuilder::new()
                .title("LemonCUC API")
                .version(env!("CARGO_PKG_VERSION"))
                .description(Some("AI-friendly desktop control API")),
        )
    });
    Json(openapi)
}
