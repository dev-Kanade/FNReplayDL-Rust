use axum::{
    Router,
    extract::Query,
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tracing_subscriber;

mod auth;
mod buffer;
mod builder;
mod constants;
mod downloader;

use auth::EpicAuth;
use downloader::ReplayDownloader;

#[derive(Deserialize)]
struct ReplayQuery {
    match_id: String,
    checkpoint: Option<bool>,
    event: Option<bool>,
    no_data: Option<bool>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = Router::new().route("/api", get(download_replay));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    println!("Fortnite Replay Downloader RustAPI が起動しました");
    println!("   エンドポイント: http://{}:3000/api", addr.ip());
    println!("   使用例: /api?match_id=xxxxxxxxxxxxxxx");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn download_replay(Query(params): Query<ReplayQuery>) -> Result<Response, String> {
    if params.match_id.trim().is_empty() {
        return Err("match_id is required".to_string());
    }

    let auth = EpicAuth::new();
    let downloader = ReplayDownloader::new(auth);

    let metadata = downloader
        .get_metadata(&params.match_id)
        .await
        .map_err(|e| format!("Failed to get metadata: {}", e))?;

    let replay_bytes = builder::build_replay(
        &metadata,
        &downloader,
        params.checkpoint.unwrap_or(false),
        params.event.unwrap_or(false),
        params.no_data.unwrap_or(false),
    )
    .await
    .map_err(|e| format!("Failed to build replay: {}", e))?;

    let filename = format!("{}.replay", params.match_id);

    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
            (
                header::CONTENT_LENGTH,
                replay_bytes.len().to_string().as_str(),
            ),
        ],
        replay_bytes,
    )
        .into_response())
}
