use leptos::prelude::*;

use crate::api::{clear_api_key, get_api_key, set_api_key};

#[component]
pub fn SettingsModal(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (api_key, set_api_key_signal) = signal(get_api_key().unwrap_or_default());

    let on_save = move |_| {
        let key = api_key.get();
        if key.is_empty() {
            clear_api_key();
        } else {
            set_api_key(&key);
        }
        on_close.run(());
    };

    let on_close_click = move |_| {
        on_close.run(());
    };

    view! {
        <dialog class="modal" class:modal-open=move || open.get()>
            <div class="modal-box">
                <h3 class="font-bold text-lg">"Settings"</h3>

                <div class="form-control w-full mt-4">
                    <label class="label">
                        <span class="label-text">"API Key"</span>
                    </label>
                    <input
                        type="password"
                        placeholder="Enter API key (leave empty if not required)"
                        class="input input-bordered w-full"
                        prop:value=move || api_key.get()
                        on:input=move |ev| {
                            set_api_key_signal.set(event_target_value(&ev));
                        }
                    />
                    <label class="label">
                        <span class="label-text-alt text-base-content/60">
                            "Required if BM_API_KEY is set on the server"
                        </span>
                    </label>
                </div>

                <div class="modal-action">
                    <button class="btn" on:click=on_close_click>
                        "Close"
                    </button>
                    <button class="btn btn-primary" on:click=on_save>
                        "Save"
                    </button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop">
                <button on:click=on_close_click>"close"</button>
            </form>
        </dialog>
    }
}
