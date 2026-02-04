use leptos::prelude::*;

use crate::api::Bookmark;

#[component]
pub fn BookmarkCard(
    bookmark: Bookmark,
    #[prop(optional)] on_delete: Option<Callback<i32>>,
) -> impl IntoView {
    let id = bookmark.id;

    view! {
        <div class="card bg-base-100 shadow-md hover:shadow-lg transition-shadow">
            <div class="card-body">
                <h2 class="card-title text-lg">
                    <a
                        href={bookmark.url.clone()}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="link link-hover"
                    >
                        {bookmark.title.clone()}
                    </a>
                </h2>

                <p class="text-sm text-base-content/60 truncate">
                    {bookmark.url.clone()}
                </p>

                {bookmark.folder.as_ref().map(|folder| view! {
                    <div class="badge badge-outline badge-sm">
                        {format!("📁 {}", folder)}
                    </div>
                })}

                <div class="flex flex-wrap gap-1 mt-2">
                    {bookmark.tags.iter().map(|tag| view! {
                        <span class="badge badge-primary badge-sm">
                            {format!("#{}", tag)}
                        </span>
                    }).collect::<Vec<_>>()}
                </div>

                <div class="card-actions justify-end mt-4">
                    <a
                        href={bookmark.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="btn btn-primary btn-sm"
                    >
                        "Open"
                    </a>
                    {on_delete.map(move |callback| view! {
                        <button
                            class="btn btn-error btn-sm btn-outline"
                            on:click=move |_| callback.run(id)
                        >
                            "Delete"
                        </button>
                    })}
                </div>
            </div>
        </div>
    }
}
