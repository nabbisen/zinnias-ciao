use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
enum AttendStatus {
    Attend,
    NotAttend,
    NotCommitted,
}

#[derive(Clone, Debug)]
struct Calendar {
    pub dates: Vec<String>,
}

#[derive(Clone, Debug)]
struct Event {
    pub date: String,
    pub place: String,
}

#[derive(Clone, Debug)]
struct Member {
    pub id: usize,
    pub name: String,
}

#[derive(Clone, Debug)]
struct MemberStatuses {
    pub member_id: usize,
    pub statuses: Vec<MemberStatus>,
}

#[derive(Clone, Debug)]
struct MemberStatus {
    pub date: String,
    pub status: AttendStatus,
    pub note: Option<String>,
}

use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn test_data() -> (Calendar, Vec<Event>, Vec<Member>, Vec<MemberStatuses>) {
    let dates = vec![
        "2026/04/06".to_owned(),
        "2026/04/13".to_owned(),
        "2026/04/20".to_owned(),
        "2026/04/27".to_owned(),
        "2026/05/04".to_owned(),
        "2026/05/11".to_owned(),
        "2026/05/18".to_owned(),
        "2026/05/25".to_owned(),
        "2026/06/01".to_owned(),
        "2026/06/08".to_owned(),
        "2026/06/15".to_owned(),
        "2026/06/22".to_owned(),
        "2026/06/29".to_owned(),
        "2026/07/06".to_owned(),
        "2026/07/13".to_owned(),
        "2026/07/20".to_owned(),
        "2026/07/27".to_owned(),
    ];

    let calender = Calendar { dates };

    let events = calender
        .dates
        .iter()
        .enumerate()
        .map(|(i, date)| Event {
            date: date.to_owned(),
            place: format!("第 {} 会議室", i),
        })
        .collect();

    let members = vec![
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "あかさた".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "なはまや".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "あかさた２".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "なはまや２".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "あかさた３".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "なはまや３".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "あかさた４".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "なはまや４".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "あかさた５".to_owned(),
        },
        Member {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: "なはまや５".to_owned(),
        },
    ];

    let member_statues = members
        .iter()
        .map(|member| MemberStatuses {
            member_id: member.id,
            statuses: calender
                .dates
                .iter()
                .enumerate()
                .map(|(i, date)| MemberStatus {
                    date: date.to_owned(),
                    status: match i % 3 {
                        0 => AttendStatus::Attend,
                        1 => AttendStatus::NotAttend,
                        2 => AttendStatus::NotCommitted,
                        _ => AttendStatus::Attend,
                    },
                    note: match i % 3 {
                        0 => None,
                        1 => Some(String::from("xx:xx に早退します")),
                        2 => Some(String::from("15 分遅れます")),
                        _ => None,
                    },
                })
                .collect(),
        })
        .collect();

    (calender, events, members, member_statues)
}

#[component]
pub fn List() -> impl IntoView {
    let (calendar, events, members, members_statuses) = test_data();

    view! {
        <table class="table table-zebra table-pin-rows">
            <thead>
                <tr>
                    <th>スタッフ</th>
                    <For
                        each=move || calendar.dates.clone()
                        key=|date| date.clone()
                        children=move |date|
                            view! {
                                <th>{date}</th>
                            }
                    />
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td></td>
                    <For
                        each=move || events.clone()
                        key=|event| event.date.clone()
                        children=move |event| {
                            view! {
                                <td>{event.place}</td>
                            }
                        }
                    />
                </tr>

                <For
                    each=move || members_statuses.clone()
                    key=|member_statuses| member_statuses.member_id
                    children=move |member_statuses| {
                        let member_name = members.clone().into_iter().find(|x| x.id == member_statuses.member_id).unwrap().name;

                        view! {
                            <tr>
                                <th>{member_name}</th>
                                <For
                                    each=move || member_statuses.statuses.clone()
                                    key=|status| status.date.clone()
                                    children=move |status| {
                                        view! {
                                            <td>
                                                <div class="flex flex-1">
                                                    <div class="flex flex-col items-start">
                                                        {match status.status {
                                                            AttendStatus::Attend => view! { <span>"参加"</span> },
                                                            AttendStatus::NotAttend => view! { <span>"不参加"</span> },
                                                            AttendStatus::NotCommitted => view! { <span>"(未回答)"</span> },
                                                        }}
                                                        <button class="btn btn-primary">"更新"</button>
                                                        {status.note.unwrap_or_default()}
                                                    </div>
                                                </div>
                                            </td>
                                        }
                                    }
                                />
                            </tr>
                        }
                    }
                />
            </tbody>
        </table>
    }
}
