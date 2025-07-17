use bearmark_types::Bookmark;
use leptos::prelude::*;

#[component]
pub fn BookmarksList(bookmarks: Vec<Bookmark>) -> impl IntoView {
    if bookmarks.is_empty() {
        return view! {
            <div class="bookmarks-list empty">
                <p class="empty-message">"No bookmarks found"</p>
            </div>
        }
        .into_any();
    }

    let bookmark_count = bookmarks.len();
    let bookmark_items = bookmarks
        .into_iter()
        .map(|bookmark| {
            let tags = bookmark.tags.join(", ");
            let folder_display = bookmark.folder.unwrap_or_else(|| "No folder".to_string());
            let created_at = bookmark
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "Invalid date".to_string());

            let tags_display = if !tags.is_empty() {
                format!("Tags: {tags}")
            } else {
                "No tags".to_string()
            };

            let url = bookmark.url.clone();
            let url_display = url.clone();
            let title = bookmark.title.clone();

            view! {
                <div class="bookmark-item">
                    <div class="bookmark-header">
                        <h3 class="bookmark-title">
                            <a href={url.clone()} target="_blank" rel="noopener noreferrer">
                                {title}
                            </a>
                        </h3>
                        <div class="bookmark-meta">
                            <span class="bookmark-folder">{folder_display}</span>
                            <span class="bookmark-created">{created_at}</span>
                        </div>
                    </div>
                    <div class="bookmark-url">
                        <a href={url} target="_blank" rel="noopener noreferrer">
                            {url_display}
                        </a>
                    </div>
                    <div class="bookmark-tags">
                        <span class="tags-info">{tags_display}</span>
                    </div>
                </div>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <div class="bookmarks-list">
            <div class="bookmarks-header">
                <h2>"Bookmarks"</h2>
                <span class="bookmarks-count">{format!("{bookmark_count} bookmarks")}</span>
            </div>
            <div class="bookmarks-items">
                {bookmark_items}
            </div>
        </div>
    }
    .into_any()
}
