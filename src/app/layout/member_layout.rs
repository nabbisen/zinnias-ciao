use leptos::prelude::*;
use leptos_router::components::Outlet;

mod footer;
mod header;

use footer::Footer;
use header::Header;

#[component]
pub fn MemberLayout() -> impl IntoView {
    view! {
        <div class="flex flex-col h-screen">
            <Header />
            <main class="flex-1 overflow-auto" style="margin-bottom: 4rem;">
                <Outlet />
            </main>
            <Footer />
        </div>
    }
}
