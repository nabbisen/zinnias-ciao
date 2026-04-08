use leptos::prelude::*;

#[component]
pub fn Event() -> impl IntoView {
    view! {
        <>
            <input type="date" class="input validator" required placeholder="Pick a date in 2025"
            min="2025-01-01" max="2025-12-31"
            title="Must be valid URL" />
            <p class="validator-hint">Must be 2025</p>

            <input type="checkbox" class="toggle validator" required title="Required" />
            <p class="validator-hint">Required</p>
        </>
    }
}
