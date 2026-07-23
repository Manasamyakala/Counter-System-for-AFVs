use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::simulator::{SimulationState, JammingType};

// Embed static files inside the executable
const INDEX_HTML: &str = include_str!("web/index.html");
const INDEX_CSS: &str = include_str!("web/index.css");
const INDEX_JS: &str = include_str!("web/index.js");

pub type SharedState = Arc<RwLock<SimulationState>>;

pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/index.css", get(serve_css))
        .route("/index.js", get(serve_js))
        .route("/ws", get(ws_handler))
        .route("/api/soft_kill", post(soft_kill_handler))
        .with_state(state)
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_css() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        INDEX_CSS,
    )
}

async fn serve_js() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        INDEX_JS,
    )
}

#[derive(Deserialize)]
struct SoftKillRequest {
    drone_id: String,
    jam_type: String,
}

async fn soft_kill_handler(
    State(state): State<SharedState>,
    Json(payload): Json<SoftKillRequest>,
) -> impl IntoResponse {
    let jam_type = match payload.jam_type.as_str() {
        "RfJamming" => JammingType::RfJamming,
        "GpsSpoofing" => JammingType::GpsSpoofing,
        "Emp" => JammingType::Emp,
        _ => return axum::http::StatusCode::BAD_REQUEST.into_response(),
    };

    let mut lock = state.write().await;
    if lock.trigger_soft_kill(&payload.drone_id, jam_type) {
        axum::http::StatusCode::OK.into_response()
    } else {
        axum::http::StatusCode::NOT_FOUND.into_response()
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: SharedState) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let serialized = {
                    let lock = state.read().await;
                    serde_json::to_string(&*lock).unwrap()
                };
                if socket.send(Message::Text(serialized.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                if let Some(Ok(Message::Close(_))) | None = msg {
                    break;
                }
            }
        }
    }
}
