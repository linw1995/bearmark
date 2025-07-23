use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: String,
    api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
    pub code: Option<u16>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "API Error: {}", self.message)
    }
}

impl std::error::Error for ApiError {}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiClient {
    pub fn new() -> Self {
        // Get base URL from current window location
        let base_url = web_sys::window()
            .and_then(|w| w.location().href().ok())
            .unwrap_or_else(|| "http://localhost:8000".to_string());

        // Remove trailing slash if present
        let base_url = if base_url.ends_with('/') {
            base_url.trim_end_matches('/').to_string()
        } else {
            base_url
        };

        Self {
            base_url: format!("{base_url}/api"),
            api_key: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    #[allow(dead_code)]
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }

    pub async fn get<T>(&self, path: &str) -> ApiResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut request = Request::get(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", api_key);
        }

        let response = request.send().await.map_err(|e| ApiError {
            message: format!("Request failed: {e}"),
            code: None,
        })?;

        if response.ok() {
            response.json::<T>().await.map_err(|e| ApiError {
                message: format!("Failed to parse response: {e}"),
                code: None,
            })
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(ApiError {
                message: error_text,
                code: Some(status),
            })
        }
    }

    #[allow(dead_code)]
    pub async fn post<T, U>(&self, path: &str, body: &T) -> ApiResult<U>
    where
        T: Serialize,
        U: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut request = Request::post(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", api_key);
        }

        let response = request
            .header("Content-Type", "application/json")
            .json(body)
            .map_err(|e| ApiError {
                message: format!("Failed to serialize request: {e}"),
                code: None,
            })?
            .send()
            .await
            .map_err(|e| ApiError {
                message: format!("Request failed: {e}"),
                code: None,
            })?;

        if response.ok() {
            response.json::<U>().await.map_err(|e| ApiError {
                message: format!("Failed to parse response: {e}"),
                code: None,
            })
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(ApiError {
                message: error_text,
                code: Some(status),
            })
        }
    }

    #[allow(dead_code)]
    pub async fn patch<T, U>(&self, path: &str, body: &T) -> ApiResult<U>
    where
        T: Serialize,
        U: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        let mut request = Request::patch(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", api_key);
        }

        let response = request
            .header("Content-Type", "application/json")
            .json(body)
            .map_err(|e| ApiError {
                message: format!("Failed to serialize request: {e}"),
                code: None,
            })?
            .send()
            .await
            .map_err(|e| ApiError {
                message: format!("Request failed: {e}"),
                code: None,
            })?;

        if response.ok() {
            response.json::<U>().await.map_err(|e| ApiError {
                message: format!("Failed to parse response: {e}"),
                code: None,
            })
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(ApiError {
                message: error_text,
                code: Some(status),
            })
        }
    }

    #[allow(dead_code)]
    pub async fn delete(&self, path: &str) -> ApiResult<String> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = Request::delete(&url);

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", api_key);
        }

        let response = request.send().await.map_err(|e| ApiError {
            message: format!("Request failed: {e}"),
            code: None,
        })?;

        if response.ok() {
            response.text().await.map_err(|e| ApiError {
                message: format!("Failed to parse response: {e}"),
                code: None,
            })
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(ApiError {
                message: error_text,
                code: Some(status),
            })
        }
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}
