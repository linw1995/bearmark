use leptos::prelude::*;

mod api;
mod app;
mod components;
mod pages;

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);

    mount_to_body(app::App);
}
