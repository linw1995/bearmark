use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="hero min-h-[60vh]">
            <div class="hero-content text-center">
                <div class="max-w-md">
                    <h1 class="text-5xl font-bold">"Bearmark"</h1>
                    <p class="py-6">
                        "A lightweight browser bookmark management system for developers who need personalized management through API integration."
                    </p>
                    <A href="/bookmarks" attr:class="btn btn-primary">
                        "View Bookmarks"
                    </A>
                </div>
            </div>
        </div>

        <div class="divider" />

        <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
            <FeatureCard
                title="Powerful Search"
                description="Use BearQL to search with keywords, tags (#rust), and folder paths (/dev)."
                icon="🔍"
            />
            <FeatureCard
                title="Organize with Tags"
                description="Add multiple tags to bookmarks for flexible categorization."
                icon="🏷️"
            />
            <FeatureCard
                title="Folder Structure"
                description="Organize bookmarks in hierarchical folders like a file system."
                icon="📁"
            />
        </div>
    }
}

#[component]
fn FeatureCard(
    title: &'static str,
    description: &'static str,
    icon: &'static str,
) -> impl IntoView {
    view! {
        <div class="card bg-base-100 shadow-md">
            <div class="card-body items-center text-center">
                <span class="text-4xl">{icon}</span>
                <h2 class="card-title">{title}</h2>
                <p>{description}</p>
            </div>
        </div>
    }
}
