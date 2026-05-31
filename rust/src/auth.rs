use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

#[derive(Clone)]
pub struct EpicAuth {
    client: Client,
    token: Arc<Mutex<Option<String>>>,
}

impl EpicAuth {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            token: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get_headers(&self) -> Result<reqwest::header::HeaderMap, String> {
        let mut token_guard = self.token.lock().await;

        if token_guard.is_none() {
            self.authenticate(&mut token_guard).await?;
        }

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            token_guard.as_ref().unwrap().parse().unwrap(),
        );
        headers.insert("User-Agent", USER_AGENT.parse().unwrap());
        Ok(headers)
    }

    async fn authenticate(
        &self,
        token: &mut tokio::sync::MutexGuard<'_, Option<String>>,
    ) -> Result<(), String> {
        let params = [("grant_type", "client_credentials"), ("token_type", "eg1")];

        let resp: TokenResponse = self
            .client
            .post(TOKEN_ENDPOINT)
            .basic_auth(AUTH_CLIENT_ID, Some(AUTH_CLIENT_SECRET))
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        *token = Some(format!("{} {}", resp.token_type, resp.access_token));
        Ok(())
    }
}
