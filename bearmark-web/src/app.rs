use leptos::prelude::*;
use leptos_meta::{Title, provide_meta_context};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::components::Navbar;
use crate::pages::{BookmarksPage, HomePage};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="Bearmark" />
        <Router>
            <div class="min-h-screen bg-base-200">
                <Navbar />
                <main class="container mx-auto px-4 py-8">
                    <Routes fallback=|| "Page not found.">
                        <Route path=path!("/") view=HomePage />
                        <Route path=path!("/bookmarks") view=BookmarksPage />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
