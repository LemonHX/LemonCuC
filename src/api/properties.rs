//! GET /api/properties – read/write /etc/lemoncuc/properties.json.

use axum::Json;
use axum_openapi3::endpoint;
use axum_openapi3::utoipa;
use axum_openapi3::utoipa::ToSchema;
use serde::{Deserialize, Serialize};

const PROPERTIES_PATH: &str = "/etc/lemoncuc/properties.json";

/// The full properties document.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Properties {
    #[serde(default)]
    pub apps: Vec<AppEntry>,
    #[serde(default)]
    pub cursor: CursorConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct CursorConfig {
    #[serde(default = "default_cursor_theme")]
    pub theme: String,
    #[serde(default = "default_cursor_size")]
    pub size: u32,
}

fn default_cursor_theme() -> String {
    "Adwaita".into()
}
fn default_cursor_size() -> u32 {
    48
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct DisplayConfig {
    #[serde(default = "default_resolution")]
    pub resolution: String,
}

fn default_resolution() -> String {
    "1280x800x24".into()
}

#[endpoint(method = "GET", path = "/api/properties", description = "Read application properties")]
pub async fn api_get_properties() -> Json<Properties> {
    match tokio::fs::read_to_string(PROPERTIES_PATH).await {
        Ok(content) => match serde_json::from_str::<Properties>(&content) {
            Ok(props) => Json(props),
            Err(_) => Json(Properties {
                apps: vec![],
                cursor: CursorConfig::default(),
                display: DisplayConfig::default(),
            }),
        },
        Err(_) => Json(Properties {
            apps: vec![],
            cursor: CursorConfig::default(),
            display: DisplayConfig::default(),
        }),
    }
}
