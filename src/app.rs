use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_meta::MetaTags;
use leptos_meta::{provide_meta_context, Stylesheet};
use leptos_router::{
    components::{ParentRoute, Router, Routes},
    path,
};

mod components;
mod layout;
mod pages;
mod router;

use layout::member_layout::MemberLayout;
use router::MemberRoutes;

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/zinnias-ciao.css"/>

        // content for this welcome page
        <Router>
            <Routes fallback=|| "Not found.">
                <ParentRoute path=path!("") view=MemberLayout>
                    <MemberRoutes />
                </ParentRoute>
            </Routes>
        </Router>
    }
}
