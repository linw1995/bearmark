use leptos::{ev::SubmitEvent, prelude::*};

#[component]
pub fn SearchBar(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_search: Callback<String>,
) -> impl IntoView {
    let input_ref: NodeRef<leptos::html::Input> = NodeRef::new();

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        if let Some(input) = input_ref.get() {
            on_search.run(input.value());
        }
    };

    view! {
        <form on:submit=on_submit class="join w-full max-w-xl">
            <input
                type="text"
                placeholder="Search bookmarks... (e.g., rust #programming /dev)"
                class="input input-bordered join-item flex-1"
                node_ref=input_ref
                prop:value=value
            />
            <button type="submit" class="btn btn-primary join-item">
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    class="h-5 w-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                    />
                </svg>
                "Search"
            </button>
        </form>
    }
}
