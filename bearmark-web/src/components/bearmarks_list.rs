use bearmark_types::Bookmark;
use leptos::prelude::*;

#[component]
pub fn BookmarksList(bookmarks: Vec<Bookmark>) -> impl IntoView {
    if bookmarks.is_empty() {
        return view! {
            <div class="max-w-4xl mx-auto p-6">
                <div class="text-center py-12">
                    <div class="mb-4">
                        <svg class="w-16 h-16 mx-auto text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"/>
                        </svg>
                    </div>
                    <p class="text-gray-500 text-lg font-medium">"No bookmarks found"</p>
                    <p class="text-gray-400 text-sm mt-2">"Start by adding your first bookmark"</p>
                </div>
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
                <div class="bg-white rounded-xl shadow-lg hover:shadow-2xl transition-all duration-300 p-6 border border-gray-200 hover:border-blue-400 group transform hover:-translate-y-1 backdrop-blur-sm">
                    <div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 mb-4">
                        <h3 class="text-lg font-semibold text-gray-900 flex-1">
                            <a href={url.clone()} target="_blank" rel="noopener noreferrer" 
                               class="text-blue-600 hover:text-blue-800 transition-colors duration-200 group-hover:text-blue-700 underline-offset-2 hover:underline">
                                {title}
                            </a>
                        </h3>
                        <div class="flex flex-col sm:flex-row sm:items-center gap-2 text-sm text-gray-500">
                            <span class="inline-flex items-center px-3 py-1 rounded-full bg-gradient-to-r from-blue-50 to-indigo-50 text-blue-700 border border-blue-200 shadow-sm">
                                <svg class="w-4 h-4 mr-1" fill="currentColor" viewBox="0 0 20 20">
                                    <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"/>
                                </svg>
                                {folder_display}
                            </span>
                            <span class="text-xs text-gray-400 bg-gray-50 px-2 py-1 rounded-md hover:bg-gray-100 transition-colors">{created_at}</span>
                        </div>
                    </div>
                    <div class="mb-4">
                        <a href={url} target="_blank" rel="noopener noreferrer" 
                           class="text-sm text-gray-600 hover:text-blue-600 transition-colors duration-200 break-all bg-gray-50 hover:bg-blue-50 px-3 py-2 rounded-lg block border border-transparent hover:border-blue-200">
                            {url_display}
                        </a>
                    </div>
                    <div class="flex flex-wrap gap-2">
                        <span class="text-xs text-gray-600 bg-gradient-to-r from-green-50 to-emerald-50 border border-green-200 px-3 py-1 rounded-full shadow-sm hover:shadow-md transition-shadow">
                            <svg class="w-3 h-3 inline mr-1" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M17.707 9.293a1 1 0 010 1.414l-7 7a1 1 0 01-1.414 0l-7-7A.997.997 0 012 10V5a3 3 0 013-3h5c.256 0 .512.098.707.293l7 7zM5 6a1 1 0 100-2 1 1 0 000 2z" clip-rule="evenodd"/>
                            </svg>
                            {tags_display}
                        </span>
                    </div>
                </div>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <div class="max-w-4xl mx-auto p-6 bg-gradient-to-br from-gray-50 to-blue-50 min-h-screen">
            <div class="flex items-center justify-between mb-8 bg-gradient-to-r from-blue-50 to-indigo-50 p-6 rounded-xl border border-blue-200 shadow-lg backdrop-blur-sm">
                <div class="flex items-center gap-3">
                    <div class="w-8 h-8 bg-gradient-to-r from-blue-600 to-blue-700 rounded-lg flex items-center justify-center shadow-md">
                        <svg class="w-5 h-5 text-white" fill="currentColor" viewBox="0 0 20 20">
                            <path d="M5 4a2 2 0 012-2h6a2 2 0 012 2v14l-5-2.5L5 18V4z"/>
                        </svg>
                    </div>
                    <h2 class="text-3xl font-bold bg-gradient-to-r from-gray-900 to-blue-800 bg-clip-text text-transparent">"Bookmarks"</h2>
                </div>
                <span class="text-sm font-medium text-blue-700 bg-white px-4 py-2 rounded-full shadow-md border border-blue-200 hover:shadow-lg transition-shadow">
                    {format!("{bookmark_count} bookmarks")}
                </span>
            </div>
            <div class="space-y-6">
                {bookmark_items}
            </div>
        </div>
    }
    .into_any()
}
