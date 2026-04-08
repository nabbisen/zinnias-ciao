use leptos::prelude::*;

use crate::app::components::show_data_from_api::ShowDataFromApi;

/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <>
            <h1>"Home"</h1>
            <ShowDataFromApi />
        </>
    }
}
