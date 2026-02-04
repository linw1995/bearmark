use gloo_net::http::RequestBuilder;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

const API_KEY_STORAGE_KEY: &str = "bearmark_api_key";

/// Error message for unauthorized requests
pub const UNAUTHORIZED_ERROR: &str = "UNAUTHORIZED";

/// Check if an error indicates unauthorized access
pub fn is_unauthorized(err: &str) -> bool {
    err == UNAUTHORIZED_ERROR
}

/// Base URL for the API - configure based on environment
fn api_base_url() -> &'static str {
    option_env!("API_BASE_URL").unwrap_or("/api")
}

/// Get API key from localStorage
pub fn get_api_key() -> Option<String> {
    LocalStorage::get(API_KEY_STORAGE_KEY).ok()
}

/// Set API key in localStorage
pub fn set_api_key(key: &str) {
    let _ = LocalStorage::set(API_KEY_STORAGE_KEY, key);
}

/// Clear API key from localStorage
pub fn clear_api_key() {
    LocalStorage::delete(API_KEY_STORAGE_KEY);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: String,
    api_key: Option<String>,
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            base_url: api_base_url().to_string(),
            api_key: get_api_key(),
        }
    }

    fn build_request(&self, method: &str, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let req = match method {
            "GET" => RequestBuilder::new(&url).method(gloo_net::http::Method::GET),
            "POST" => RequestBuilder::new(&url).method(gloo_net::http::Method::POST),
            "PATCH" => RequestBuilder::new(&url).method(gloo_net::http::Method::PATCH),
            "DELETE" => RequestBuilder::new(&url).method(gloo_net::http::Method::DELETE),
            _ => RequestBuilder::new(&url).method(gloo_net::http::Method::GET),
        };

        if let Some(ref key) = self.api_key {
            req.header("Authorization", key)
        } else {
            req
        }
    }

    fn build_list_bookmarks_path(query: Option<&str>, limit: Option<u32>) -> String {
        let mut path = "/bookmarks".to_string();
        let mut params = Vec::new();

        if let Some(q) = query
            && !q.is_empty()
        {
            params.push(format!("q={}", urlencoding::encode(q)));
        }
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }

        if !params.is_empty() {
            path = format!("{}?{}", path, params.join("&"));
        }
        path
    }

    pub async fn list_bookmarks(
        &self,
        query: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Bookmark>, String> {
        let path = Self::build_list_bookmarks_path(query, limit);

        let response = self
            .build_request("GET", &path)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status() == 401 || response.status() == 403 {
            return Err(UNAUTHORIZED_ERROR.to_string());
        }
        if !response.ok() {
            return Err(format!("API error: {}", response.status()));
        }

        response.json().await.map_err(|e| e.to_string())
    }

    pub async fn delete_bookmark(&self, id: i32) -> Result<(), String> {
        let response = self
            .build_request("DELETE", &format!("/bookmarks/{}", id))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.ok() {
            return Err(format!("API error: {}", response.status()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_unauthorized() {
        assert!(is_unauthorized(UNAUTHORIZED_ERROR));
        assert!(!is_unauthorized("some other error"));
        assert!(!is_unauthorized(""));
    }

    #[test]
    fn test_build_list_bookmarks_path_no_params() {
        let path = ApiClient::build_list_bookmarks_path(None, None);
        assert_eq!(path, "/bookmarks");
    }

    #[test]
    fn test_build_list_bookmarks_path_with_query() {
        let path = ApiClient::build_list_bookmarks_path(Some("rust"), None);
        assert_eq!(path, "/bookmarks?q=rust");
    }

    #[test]
    fn test_build_list_bookmarks_path_with_empty_query() {
        let path = ApiClient::build_list_bookmarks_path(Some(""), None);
        assert_eq!(path, "/bookmarks");
    }

    #[test]
    fn test_build_list_bookmarks_path_with_limit() {
        let path = ApiClient::build_list_bookmarks_path(None, Some(10));
        assert_eq!(path, "/bookmarks?limit=10");
    }

    #[test]
    fn test_build_list_bookmarks_path_with_query_and_limit() {
        let path = ApiClient::build_list_bookmarks_path(Some("rust"), Some(20));
        assert_eq!(path, "/bookmarks?q=rust&limit=20");
    }

    #[test]
    fn test_build_list_bookmarks_path_encodes_special_chars() {
        let path = ApiClient::build_list_bookmarks_path(Some("hello world"), None);
        assert_eq!(path, "/bookmarks?q=hello%20world");

        let path = ApiClient::build_list_bookmarks_path(Some("#tag"), None);
        assert_eq!(path, "/bookmarks?q=%23tag");
    }

    #[test]
    fn test_bookmark_deserialize() {
        let json = r#"{
            "id": 1,
            "title": "Test",
            "url": "https://example.com",
            "folder": null,
            "tags": [],
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "deleted_at": null
        }"#;
        let bookmark: Bookmark = serde_json::from_str(json).unwrap();
        assert_eq!(bookmark.id, 1);
        assert_eq!(bookmark.title, "Test");
        assert_eq!(bookmark.url, "https://example.com");
        assert!(bookmark.folder.is_none());
        assert!(bookmark.tags.is_empty());
        assert!(bookmark.deleted_at.is_none());
    }

    #[test]
    fn test_bookmark_deserialize_with_folder_and_tags() {
        let json = r#"{
            "id": 1,
            "title": "Test",
            "url": "https://example.com",
            "folder": "/dev",
            "tags": ["rust", "web"],
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "deleted_at": null
        }"#;
        let bookmark: Bookmark = serde_json::from_str(json).unwrap();
        assert_eq!(bookmark.folder.as_ref().unwrap(), "/dev");
        assert_eq!(bookmark.tags.len(), 2);
        assert_eq!(bookmark.tags[0], "rust");
    }

    #[test]
    fn test_bookmark_deserialize_api_response_format() {
        // Test with actual API response format (RFC3339 timestamps)
        let json = r#"{
            "id": 42,
            "title": "Rust Programming Language",
            "url": "https://www.rust-lang.org/",
            "folder": "/programming/languages",
            "tags": ["rust", "programming", "systems"],
            "created_at": "2024-06-15T10:30:00+00:00",
            "updated_at": "2024-06-20T14:45:30+00:00",
            "deleted_at": null
        }"#;
        let bookmark: Bookmark = serde_json::from_str(json).unwrap();
        assert_eq!(bookmark.id, 42);
        assert_eq!(bookmark.title, "Rust Programming Language");
        assert_eq!(bookmark.folder, Some("/programming/languages".to_string()));
        assert_eq!(bookmark.tags, vec!["rust", "programming", "systems"]);
        assert!(bookmark.deleted_at.is_none());
    }

    #[test]
    fn test_bookmark_list_deserialize() {
        // Test deserializing a list of bookmarks (API returns Vec<Bookmark>)
        let json = r#"[
            {
                "id": 1,
                "title": "First",
                "url": "https://first.com",
                "folder": null,
                "tags": [],
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "deleted_at": null
            },
            {
                "id": 2,
                "title": "Second",
                "url": "https://second.com",
                "folder": "/test",
                "tags": ["tag1"],
                "created_at": "2024-01-02T00:00:00Z",
                "updated_at": "2024-01-02T00:00:00Z",
                "deleted_at": null
            }
        ]"#;
        let bookmarks: Vec<Bookmark> = serde_json::from_str(json).unwrap();
        assert_eq!(bookmarks.len(), 2);
        assert_eq!(bookmarks[0].id, 1);
        assert_eq!(bookmarks[1].folder, Some("/test".to_string()));
    }
}
