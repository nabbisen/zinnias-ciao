use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route},
    path, MatchNestedRoutes,
};

use super::pages::{event::Event, home_page::HomePage, list::List, settings::Settings};

#[component(transparent)]
pub fn MemberRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("") view=|| view! { <Outlet /> }>
            <Route path=path!("") view=HomePage/>
            <Route path=path!("list") view=List/>
            <Route path=path!("update") view=Event/>
            <Route path=path!("settings") view=Settings/>
        </ParentRoute>
    }
    .into_inner()
}
