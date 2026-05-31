use crate::auth::EpicAuth;
use crate::constants::*;
use bytes::Bytes;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct ReplayDownloader {
    auth: EpicAuth,
    client: Client,
}

impl ReplayDownloader {
    pub fn new(auth: EpicAuth) -> Self {
        Self {
            auth,
            client: Client::builder()
                .user_agent(USER_AGENT)
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    pub async fn get_metadata(&self, match_id: &str) -> Result<Value, String> {
        let headers = self
            .auth
            .get_headers()
            .await
            .map_err(|e| format!("認証エラー: {}", e))?;

        let url = format!("{META_DATA_URL}{match_id}.json");

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status() != StatusCode::OK {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Metadata fetch failed: {} - {}", status, body));
        }

        resp.json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    pub async fn get_download_links(
        &self,
        match_id: &str,
        filenames: Vec<String>,
    ) -> Result<HashMap<String, Value>, String> {
        let headers = self
            .auth
            .get_headers()
            .await
            .map_err(|e| format!("認証エラー: {}", e))?;

        let url = format!("{BASE_DATA_URL}{match_id}/");

        let payload = serde_json::json!({ "files": filenames });

        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("リクエストエラー: {}", e))?;

        if resp.status() != StatusCode::OK {
            return Err(format!("ダウンロードリンク取得エラー: HTTP {}", resp.status()));
        }

        let data: Value = resp.json().await.map_err(|e| e.to_string())?;

        let files = data["files"]
            .as_object()
            .ok_or("レスポンス解析エラー: 'files' が見つかりません")?
            .clone();

        Ok(files.into_iter().map(|(k, v)| (k, v)).collect())
    }

    pub async fn download_chunk(&self, url: &str, file_id: &str) -> Result<Bytes, String> {
        let headers = self
            .auth
            .get_headers()
            .await
            .map_err(|e| format!("ヘッダー取得エラー: {}", e))?;

        info!("ダウンロード中: {} (ID: {})", url, file_id);

        let resp = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("ダウンロード失敗 {}: {}", file_id, e))?;

        if resp.status() != StatusCode::OK {
            return Err(format!("HTTP {} for chunk {}", resp.status(), file_id));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("ダウンロード失敗 {}: {}", file_id, e))?;

        info!("ダウンロード完了 {}: {} bytes", file_id, bytes.len());
        Ok(bytes)
    }

    pub async fn download_chunks_parallel(
        &self,
        links: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Bytes>, String> {
        let mut tasks = Vec::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(8));

        for (filename, info) in links {
            let url = info["readLink"]
                .as_str()
                .ok_or("ダウンロードリンク取得エラー: 'readLink' がありません")?
                .to_string();

            let file_id = filename.clone();
            let downloader = self.clone();
            let permit = semaphore.clone();

            let task = tokio::spawn(async move {
                let _permit = permit.acquire().await.unwrap();
                match downloader.download_chunk(&url, &file_id).await {
                    Ok(data) => Ok((file_id, data)),
                    Err(e) => {
                        error!("ダウンロード失敗 {}: {}", file_id, e);
                        Err(e)
                    }
                }
            });

            tasks.push(task);
        }

        let mut results = HashMap::new();

        for task in tasks {
            match task.await {
                Ok(Ok((id, data))) => {
                    results.insert(id, data);
                }
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(format!("タスクJoin失敗: {}", e)),
            }
        }

        Ok(results)
    }
}
