use crate::components::BookmarksList;
use bearmark_types::Bookmark;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    // 创建示例数据
    let sample_bookmarks = vec![
        Bookmark {
            id: 1,
            title: "Rust Programming Language".to_string(),
            url: "https://www.rust-lang.org/".to_string(),
            folder: Some("/dev/rust".to_string()),
            tags: vec!["rust".to_string(), "programming".to_string()],
            created_at: time::OffsetDateTime::now_utc(),
            deleted_at: None,
            updated_at: time::OffsetDateTime::now_utc(),
        },
        Bookmark {
            id: 2,
            title: "Leptos Framework".to_string(),
            url: "https://leptos.dev/".to_string(),
            folder: Some("/dev/web".to_string()),
            tags: vec!["leptos".to_string(), "web".to_string(), "rust".to_string()],
            created_at: time::OffsetDateTime::now_utc(),
            deleted_at: None,
            updated_at: time::OffsetDateTime::now_utc(),
        },
        Bookmark {
            id: 3,
            title: "GitHub".to_string(),
            url: "https://github.com/".to_string(),
            folder: None,
            tags: vec!["git".to_string(), "development".to_string()],
            created_at: time::OffsetDateTime::now_utc(),
            deleted_at: None,
            updated_at: time::OffsetDateTime::now_utc(),
        },
    ];

    view! {
        <div class="app">
            <header class="app-header">
                <h1>"Bearmark"</h1>
                <p>"Bookmark Management System"</p>
            </header>
            <main class="app-main">
                <BookmarksList bookmarks={sample_bookmarks} />
            </main>
        </div>
    }
}
