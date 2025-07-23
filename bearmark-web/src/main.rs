mod api;
mod app;
mod components;

use app::App;

fn main() {
    leptos::mount::mount_to_body(App)
}
