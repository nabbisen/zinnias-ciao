use chrono::{Datelike, NaiveDate};
use leptos::prelude::*;

#[derive(Clone, Debug)]
struct Event {
    title: String,
    day: u32,
}

fn get_calendar_days(year: i32, month: u32) -> Vec<Option<u32>> {
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid date");
    let first_weekday = first_day.weekday().num_days_from_sunday();

    let last_day = match month {
        12 => NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .expect("Invalid date")
            .pred_opt()
            .expect("Failed to get previous day")
            .day(),
        _ => NaiveDate::from_ymd_opt(year, month + 1, 1)
            .expect("Invalid date")
            .pred_opt()
            .expect("Failed to get previous day")
            .day(),
    };

    let mut days = Vec::with_capacity((first_weekday + last_day) as usize);

    for _ in 0..first_weekday {
        days.push(None);
    }

    for d in 1..=last_day {
        days.push(Some(d));
    }

    days
}

fn today() -> chrono::NaiveDate {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Date;

        let d = Date::new_0();
        return chrono::NaiveDate::from_ymd_opt(
            d.get_full_year() as i32,
            (d.get_month() + 1) as u32,
            d.get_date() as u32,
        )
        .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        return chrono::Local::now().date_naive();
    }
}

#[component]
pub fn Calendar() -> impl IntoView {
    let today = today();

    let (year, set_year) = signal(today.year());
    let (month, set_month) = signal(today.month() as u32);

    let (events, set_events) = signal(vec![
        Event {
            title: "Meeting".to_string(),
            day: 5,
        },
        Event {
            title: "Lunch".to_string(),
            day: 5,
        },
        Event {
            title: "Deploy".to_string(),
            day: 12,
        },
    ]);

    let days = move || get_calendar_days(year.get(), month.get());

    let events_for_day = move |day: u32| {
        events
            .get()
            .iter()
            .filter(|e| e.day == day)
            .cloned()
            .collect::<Vec<_>>()
    };

    let prev_month = move |_| {
        if month.get() == 1 {
            set_month.set(12);
            set_year.update(|y| *y -= 1);
        } else {
            set_month.update(|m| *m -= 1);
        }
    };

    let next_month = move |_| {
        if month.get() == 12 {
            set_month.set(1);
            set_year.update(|y| *y += 1);
        } else {
            set_month.update(|m| *m += 1);
        }
    };

    view! {
        <div class="w-full max-w-3xl p-4 rounded-xl bg-base-100 shadow-md">
            <div class="flex justify-between items-center mb-4">
                <button
                    class="btn btn-sm btn-neutral"
                    on:click=prev_month
                >
                    "❮"
                </button>

                <div class="text-lg font-bold">
                    {move || format!("{} / {}", year.get(), month.get())}
                </div>

                <button
                    class="btn btn-sm btn-neutral"
                    on:click=next_month
                >
                    "❯"
                </button>
            </div>

            <div class="grid grid-cols-7 text-xs opacity-60 mb-1">
                <div>"Sun"</div>
                <div>"Mon"</div>
                <div>"Tue"</div>
                <div>"Wed"</div>
                <div>"Thu"</div>
                <div>"Fri"</div>
                <div>"Sat"</div>
            </div>

            <div class="grid grid-cols-7 gap-1">
                {move || {
                    days()
                        .into_iter()
                        .map(|d| {
                            view! {
                                <div class="h-24 border rounded-lg relative bg-base-200 p-1">
                                    <div class="text-xs p-1 font-bold">
                                        {d.map(|v| v.to_string()).unwrap_or_default()}
                                    </div>

                                    <div class="absolute top-6 left-1 right-1 bottom-1 overflow-hidden flex flex-col gap-1">
                                        {d.map(|day| {
                                            events_for_day(day)
                                                .into_iter()
                                                .map(|e| {
                                                    view! {
                                                        <div class="badge badge-primary badge-sm truncate text-xs">
                                                            {e.title.clone()}
                                                        </div>
                                                    }
                                                })
                                                .collect::<Vec<_>>()
                                                .into_view()
                                        })}
                                    </div>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_view()
                }}
            </div>
        </div>
    }
}
