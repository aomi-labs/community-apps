mod ai;
mod alerts;
mod aomi;
mod base;
mod engine;
mod market;
mod wallet;

use axum::{
    Json, Router,
    routing::{get, post},
};
use engine::process_message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Serialize)]
struct ChatResponse {
    reply: String,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "AlphaScout Agent".to_string(),
    })
}

async fn chat_handler(Json(payload): Json<ChatRequest>) -> Json<ChatResponse> {
    let reply = process_message(payload.message).await;
    Json(ChatResponse { reply })
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(chat_handler))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    println!("🚀 AlphaScout backend running on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    alerts::start_alert_loop().await;

    axum::serve(listener, app).await.unwrap();
}
