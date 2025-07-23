use crate::api::{ApiClient, fetch_bookmarks};
use crate::components::BookmarksList;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    // Create API client
    let client = ApiClient::new();

    // Create a refresh trigger
    let (refresh_trigger, set_refresh_trigger) = signal(0);

    // Create a resource for fetching bookmarks
    let bookmarks_resource = LocalResource::new(move || {
        let _trigger = refresh_trigger.get();
        let client = client.clone();
        async move { fetch_bookmarks(&client, None).await }
    });

    // Refresh function
    let refresh_bookmarks = move |_| {
        set_refresh_trigger.update(|n| *n += 1);
    };

    view! {
        <div class="app">
            <header class="app-header bg-gradient-to-r from-blue-600 to-blue-800 text-white p-6 shadow-lg">
                <div class="max-w-4xl mx-auto flex items-center justify-between">
                    <div>
                        <h1 class="text-3xl font-bold">"Bearmark"</h1>
                        <p class="text-blue-100 mt-1">"Bookmark Management System"</p>
                    </div>
                    <button
                        class="bg-white text-blue-600 hover:bg-blue-50 px-4 py-2 rounded-lg font-medium transition-colors duration-200 flex items-center gap-2 shadow-md hover:shadow-lg"
                        on:click=refresh_bookmarks
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                        </svg>
                        "Refresh"
                    </button>
                </div>
            </header>
            <main class="app-main">
                <Suspense fallback=|| view! {
                    <div class="max-w-4xl mx-auto p-6">
                        <div class="text-center py-12">
                            <div class="mb-4">
                                <svg class="w-16 h-16 mx-auto text-gray-300 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                                </svg>
                            </div>
                            <p class="text-gray-500 text-lg font-medium">"Loading bookmarks..."</p>
                        </div>
                    </div>
                }>
                    {move || {
                        bookmarks_resource.get().map(|result| {
                            match result {
                                Ok(bookmarks) => view! {
                                    <BookmarksList bookmarks={bookmarks} />
                                }.into_any(),
                                Err(err) => view! {
                                    <div class="max-w-4xl mx-auto p-6">
                                        <div class="text-center py-12">
                                            <div class="mb-4">
                                                <svg class="w-16 h-16 mx-auto text-red-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"/>
                                                </svg>
                                            </div>
                                            <p class="text-red-500 text-lg font-medium">"Error loading bookmarks"</p>
                                            <p class="text-gray-400 text-sm mt-2">{err.to_string()}</p>
                                            <div class="mt-6">
                                                <button
                                                    class="bg-red-500 hover:bg-red-600 text-white px-6 py-2 rounded-lg font-medium transition-colors duration-200 flex items-center gap-2 mx-auto shadow-md hover:shadow-lg"
                                                    on:click=refresh_bookmarks
                                                >
                                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                                                    </svg>
                                                    "Retry"
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        })
                    }}
                </Suspense>
            </main>
        </div>
    }
}
