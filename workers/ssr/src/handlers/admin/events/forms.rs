use crate::db::event as event_db;
use crate::render;
use zinnias_ciao_contracts::i18n;

use super::policy::event_is_recurring;
use super::summary::render_schedule_summary;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RepeatFieldPrefill {
    pub(super) repeat_rule: String,
    pub(super) repeat_end_mode: String,
    pub(super) repeat_count: Option<u32>,
    pub(super) repeat_until: Option<String>,
}

impl RepeatFieldPrefill {
    pub(super) fn normal_create_default() -> Self {
        Self {
            repeat_rule: "none".to_string(),
            repeat_end_mode: "open_ended".to_string(),
            repeat_count: None,
            repeat_until: None,
        }
    }
}

pub(super) fn render_event_create_fields(
    title: Option<&str>,
    location: Option<&str>,
    description: Option<&str>,
    error: Option<&str>,
    day_date: Option<&str>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
) -> String {
    render_event_create_fields_with_repeat(
        title,
        location,
        description,
        error,
        day_date,
        starts_at,
        ends_at,
        &RepeatFieldPrefill::normal_create_default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_event_create_fields_with_repeat(
    title: Option<&str>,
    location: Option<&str>,
    description: Option<&str>,
    error: Option<&str>,
    day_date: Option<&str>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
    repeat: &RepeatFieldPrefill,
) -> String {
    format!(
        "{err}\
         {title}\
         {date}\
         {start}\
         {end}\
         {loc}\
         {repeat}\
         {desc}",
        err = render_error_html(error),
        title = form_field(
            i18n::JA_FORM_FIELD_TITLE,
            "title",
            "text",
            title.unwrap_or(""),
            true
        ),
        date = form_field(
            i18n::JA_FORM_FIELD_DATE,
            "day_date",
            "date",
            day_date.unwrap_or(""),
            true
        ),
        start = form_field(
            i18n::JA_FORM_FIELD_START,
            "starts_at",
            "time",
            starts_at.unwrap_or(""),
            true
        ),
        end = form_field(
            i18n::JA_FORM_FIELD_END,
            "ends_at",
            "time",
            ends_at.unwrap_or(""),
            true
        ),
        loc = form_field(
            i18n::JA_FORM_FIELD_LOCATION,
            "location",
            "text",
            location.unwrap_or(""),
            false
        ),
        repeat = render_repeat_fields(repeat),
        desc = description_field(description),
    )
}

pub(super) fn render_recreate_event_create_fields(
    event: &event_db::EventRow,
    error: Option<&str>,
) -> String {
    format!(
        "<input type=\"hidden\" name=\"copy_source_event_id\" value=\"{eid}\">\
         <input type=\"hidden\" name=\"copy_mode\" value=\"cancelled_recreate\">\
         <p role=\"note\" style=\"font-size:.875rem;color:#6E6E73;line-height:1.5;\
         margin:0 0 1rem\">{helper}</p>\
         {fields}",
        eid = render::escape_html(&event.id),
        helper = i18n::JA_ADMIN_RECREATE_EVENT_HELPER,
        fields = render_event_create_fields(
            Some(&event.title),
            event.location.as_deref(),
            event.description.as_deref(),
            error,
            None,
            None,
            None,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_single_day_edit_fields(
    title: Option<&str>,
    location: Option<&str>,
    description: Option<&str>,
    error: Option<&str>,
    day_date: Option<&str>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
) -> String {
    format!(
        "{err}\
         {title}\
         {date}\
         {start}\
         {end}\
         {loc}\
         {desc}",
        err = render_error_html(error),
        title = form_field(
            i18n::JA_FORM_FIELD_TITLE,
            "title",
            "text",
            title.unwrap_or(""),
            true
        ),
        date = form_field(
            i18n::JA_FORM_FIELD_DATE,
            "day_date",
            "date",
            day_date.unwrap_or(""),
            true
        ),
        start = form_field(
            i18n::JA_FORM_FIELD_START,
            "starts_at",
            "time",
            starts_at.unwrap_or(""),
            true
        ),
        end = form_field(
            i18n::JA_FORM_FIELD_END,
            "ends_at",
            "time",
            ends_at.unwrap_or(""),
            true
        ),
        loc = form_field(
            i18n::JA_FORM_FIELD_LOCATION,
            "location",
            "text",
            location.unwrap_or(""),
            false
        ),
        desc = description_field(description),
    )
}

pub(super) fn render_details_only_event_edit_fields(
    event: &event_db::EventRow,
    days: &[event_db::EventDayRow],
    community_tz: &str,
    error: Option<&str>,
) -> String {
    let is_recurring = event_is_recurring(event);
    let helper = if is_recurring {
        i18n::JA_ADMIN_EDIT_RECURRING_HELPER
    } else {
        i18n::JA_ADMIN_EDIT_MULTI_DAY_HELPER
    };

    format!(
        "{err}\
         {summary}\
         <section style=\"margin:1.25rem 0 1rem\">\
         <h2 style=\"font-size:1rem;font-weight:700;margin:0 0 .5rem\">{heading}</h2>\
         <p style=\"font-size:.875rem;color:#6e6e73;line-height:1.5;margin:.25rem 0 1rem\">\
         {helper}</p>\
         <p style=\"font-size:.8125rem;color:#6e6e73;line-height:1.5;margin:.25rem 0 1rem\">\
         {preserved}</p>\
         {title}{loc}{desc}</section>",
        err = render_error_html(error),
        summary = render_schedule_summary(days, community_tz),
        heading = i18n::JA_ADMIN_EDIT_DETAILS_ONLY_HEADING,
        helper = helper,
        preserved = i18n::JA_ADMIN_EDIT_RESPONSES_PRESERVED,
        title = form_field(
            i18n::JA_FORM_FIELD_TITLE,
            "title",
            "text",
            &event.title,
            true
        ),
        loc = form_field(
            i18n::JA_FORM_FIELD_LOCATION,
            "location",
            "text",
            event.location.as_deref().unwrap_or(""),
            false
        ),
        desc = description_field(event.description.as_deref()),
    )
}

fn render_error_html(error: Option<&str>) -> String {
    error
        .map(|e| {
            format!(
                "<p role=\"alert\" style=\"color:#FF3B30;font-size:.875rem\">{}</p>",
                render::escape_html(e)
            )
        })
        .unwrap_or_default()
}

fn form_field(label: &str, name: &str, ftype: &str, val: &str, required: bool) -> String {
    let req_attr = if required { " required" } else { "" };
    format!(
        "<label style=\"display:block;margin-bottom:1rem\">\
         <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">{label}</span>\
         <input type=\"{ftype}\" name=\"{name}\" value=\"{val}\" \
           style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
           border-radius:12px;font-size:1rem\"{req_attr}>\
         </label>",
        label = label,
        ftype = ftype,
        name = name,
        val = render::escape_html(val),
    )
}

fn description_field(description: Option<&str>) -> String {
    let dval = render::escape_html(description.unwrap_or(""));
    format!(
        "<label style=\"display:block;margin-bottom:1rem\">\
         <span style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">\
         {desc_lbl}</span>\
         <textarea name=\"description\" rows=\"3\" \
           style=\"width:100%;padding:.75rem;border:1px solid #e5e5ea;\
           border-radius:12px;font-size:1rem\">{dval}</textarea>\
         </label>",
        desc_lbl = i18n::JA_FORM_FIELD_DESC,
    )
}

fn render_repeat_fields(repeat: &RepeatFieldPrefill) -> String {
    format!(
        "<div style=\"margin-bottom:1rem\">\
         <label style=\"font-size:.875rem;display:block;margin-bottom:.375rem\">{repeat_lbl}</label>\
         <div style=\"display:flex;gap:.75rem;align-items:center;flex-wrap:wrap\">\
           <select name=\"repeat_rule\" style=\"padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem;flex:1 1 10rem;min-width:0;max-width:100%\">\
             {opt_none}\
             {opt_weekly}\
             {opt_biweekly}\
             {opt_monthly}\
           </select>\
           <select name=\"repeat_end_mode\" style=\"padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem;flex:1 1 10rem;min-width:0;max-width:100%\">\
             {end_open}\
             {end_until}\
             {end_count}\
           </select>\
           <input type=\"number\" name=\"repeat_count\" value=\"{repeat_count}\" min=\"1\" max=\"52\"\
             placeholder=\"{count_ph}\" aria-label=\"{count_lbl}\"\
             style=\"width:6rem;max-width:100%;padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem\">\
           <input type=\"date\" name=\"repeat_until\" value=\"{repeat_until}\" aria-label=\"{until_lbl}\"\
             style=\"width:10rem;max-width:100%;padding:.625rem;border:1px solid #e5e5ea;\
             border-radius:12px;font-size:1rem\">\
         </div>\
         <p style=\"font-size:.75rem;color:#6e6e73;margin:.25rem 0 0\">{hint}</p>\
         </div>",
        repeat_lbl = i18n::JA_REPEAT_LABEL,
        opt_none = option_html("none", i18n::JA_REPEAT_NONE, &repeat.repeat_rule),
        opt_weekly = option_html("weekly", i18n::JA_REPEAT_WEEKLY, &repeat.repeat_rule),
        opt_biweekly = option_html("biweekly", i18n::JA_REPEAT_BIWEEKLY, &repeat.repeat_rule),
        opt_monthly = option_html("monthly", i18n::JA_REPEAT_MONTHLY, &repeat.repeat_rule),
        end_open = option_html(
            "open_ended",
            i18n::JA_REPEAT_END_OPEN,
            &repeat.repeat_end_mode
        ),
        end_until = option_html(
            "until_date",
            i18n::JA_REPEAT_END_UNTIL,
            &repeat.repeat_end_mode
        ),
        end_count = option_html(
            "after_count",
            i18n::JA_REPEAT_END_COUNT,
            &repeat.repeat_end_mode
        ),
        repeat_count = repeat
            .repeat_count
            .map(|n| n.to_string())
            .unwrap_or_default(),
        repeat_until = repeat
            .repeat_until
            .as_deref()
            .map(render::escape_html)
            .unwrap_or_default(),
        count_ph = i18n::JA_REPEAT_COUNT_UNIT,
        count_lbl = i18n::JA_REPEAT_COUNT_LABEL,
        until_lbl = i18n::JA_REPEAT_UNTIL_LABEL,
        hint = i18n::JA_REPEAT_COUNT_HINT,
    )
}

fn option_html(value: &str, label: &str, selected_value: &str) -> String {
    let selected = if value == selected_value {
        " selected"
    } else {
        ""
    };
    format!(
        "<option value=\"{}\"{}>{}</option>",
        render::escape_html(value),
        selected,
        render::escape_html(label)
    )
}
