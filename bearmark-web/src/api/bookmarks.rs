use crate::api::client::{ApiClient, ApiResult};
use bearmark_types::Bookmark;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookmark {
    pub title: String,
    pub url: String,
    pub folder_id: Option<i64>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyBookmark {
    pub title: Option<String>,
    pub url: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkSearchParams {
    pub q: Option<String>,
    pub cwd: Option<String>,
    pub before: Option<i64>,
    pub limit: Option<i64>,
}

impl BookmarkSearchParams {
    pub fn new() -> Self {
        Self {
            q: None,
            cwd: None,
            before: None,
            limit: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_query(mut self, query: String) -> Self {
        self.q = Some(query);
        self
    }

    #[allow(dead_code)]
    pub fn with_cwd(mut self, cwd: String) -> Self {
        self.cwd = Some(cwd);
        self
    }

    #[allow(dead_code)]
    pub fn with_before(mut self, before: i64) -> Self {
        self.before = Some(before);
        self
    }

    #[allow(dead_code)]
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();

        if let Some(q) = &self.q {
            params.push(format!("q={}", urlencoding::encode(q)));
        }
        if let Some(cwd) = &self.cwd {
            params.push(format!("cwd={}", urlencoding::encode(cwd)));
        }
        if let Some(before) = self.before {
            params.push(format!("before={before}"));
        }
        if let Some(limit) = self.limit {
            params.push(format!("limit={limit}"));
        }

        if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        }
    }
}

impl Default for BookmarkSearchParams {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn fetch_bookmarks(
    client: &ApiClient,
    params: Option<BookmarkSearchParams>,
) -> ApiResult<Vec<Bookmark>> {
    let query_string = params.unwrap_or_default().to_query_string();
    let path = format!("/bookmarks/{query_string}");
    client.get(&path).await
}

#[allow(dead_code)]
pub async fn create_bookmark(client: &ApiClient, bookmark: CreateBookmark) -> ApiResult<Bookmark> {
    client.post("/bookmarks/", &bookmark).await
}

#[allow(dead_code)]
pub async fn update_bookmark(
    client: &ApiClient,
    id: i64,
    bookmark: ModifyBookmark,
) -> ApiResult<Bookmark> {
    let path = format!("/bookmarks/{id}");
    client.patch(&path, &bookmark).await
}

#[allow(dead_code)]
pub async fn delete_bookmark(client: &ApiClient, id: i64) -> ApiResult<String> {
    let path = format!("/bookmarks/{id}");
    client.delete(&path).await
}
