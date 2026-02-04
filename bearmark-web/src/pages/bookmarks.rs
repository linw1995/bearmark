use leptos::prelude::*;

use crate::api::{ApiClient, Bookmark, is_unauthorized};
use crate::components::{BookmarkCard, SearchBar, SettingsModal};

#[component]
pub fn BookmarksPage() -> impl IntoView {
    let (search_query, set_search_query) = signal(String::new());
    let (bookmarks, set_bookmarks) = signal(Vec::<Bookmark>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(Option::<String>::None);
    let (settings_open, set_settings_open) = signal(false);

    let fetch_bookmarks = move |query: String| {
        set_loading.set(true);
        set_error.set(None);

        leptos::task::spawn_local(async move {
            let client = ApiClient::new();
            let query_param = if query.is_empty() {
                None
            } else {
                Some(query.as_str())
            };

            match client.list_bookmarks(query_param, Some(50)).await {
                Ok(items) => {
                    set_bookmarks.set(items);
                    set_loading.set(false);
                }
                Err(e) => {
                    if is_unauthorized(&e) {
                        set_settings_open.set(true);
                    }
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Initial fetch
    Effect::new(move || {
        fetch_bookmarks(String::new());
    });

    let on_search = Callback::new(move |query: String| {
        set_search_query.set(query.clone());
        fetch_bookmarks(query);
    });

    let on_delete = Callback::new(move |id: i32| {
        leptos::task::spawn_local(async move {
            let client = ApiClient::new();
            if client.delete_bookmark(id).await.is_ok() {
                set_bookmarks.update(|items| {
                    items.retain(|b| b.id != id);
                });
            }
        });
    });

    let on_settings_close = Callback::new(move |_| {
        set_settings_open.set(false);
        // Retry fetch after settings closed
        fetch_bookmarks(search_query.get());
    });

    let is_unauthorized_error = move || error.get().map(|e| is_unauthorized(&e)).unwrap_or(false);

    view! {
        <div class="space-y-6">
            <div class="flex flex-col items-center gap-4">
                <h1 class="text-3xl font-bold">"Bookmarks"</h1>
                <SearchBar value=search_query on_search=on_search />
            </div>

            <Show when=move || loading.get()>
                <div class="flex justify-center py-12">
                    <span class="loading loading-spinner loading-lg" />
                </div>
            </Show>

            <Show when=move || error.get().is_some() && !is_unauthorized_error()>
                <div class="alert alert-error">
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        class="stroke-current shrink-0 h-6 w-6"
                        fill="none"
                        viewBox="0 0 24 24"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
                        />
                    </svg>
                    <span>{move || error.get().unwrap_or_default()}</span>
                </div>
            </Show>

            <Show when=is_unauthorized_error>
                <div class="alert alert-warning">
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        class="stroke-current shrink-0 h-6 w-6"
                        fill="none"
                        viewBox="0 0 24 24"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                        />
                    </svg>
                    <span>"Authentication required. Please configure your API key."</span>
                    <button class="btn btn-sm" on:click=move |_| set_settings_open.set(true)>
                        "Open Settings"
                    </button>
                </div>
            </Show>

            <Show when=move || !loading.get() && error.get().is_none()>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    <For
                        each=move || bookmarks.get()
                        key=|bookmark| bookmark.id
                        children=move |bookmark| {
                            view! { <BookmarkCard bookmark=bookmark on_delete=on_delete /> }
                        }
                    />
                </div>

                <Show when=move || bookmarks.get().is_empty()>
                    <div class="text-center py-12">
                        <p class="text-base-content/60">"No bookmarks found."</p>
                    </div>
                </Show>
            </Show>
        </div>

        <SettingsModal open=settings_open on_close=on_settings_close />
    }
}
