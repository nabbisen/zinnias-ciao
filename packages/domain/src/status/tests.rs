use super::*;

fn ok(role: Role, state: DayTimeState, req: Option<AttendanceStatus>) {
    assert!(
        validate_status_transition(role, state, false, None, req).is_ok(),
        "expected OK for {role:?} + {state:?} + {req:?}"
    );
}

fn err(role: Role, state: DayTimeState, req: Option<AttendanceStatus>) {
    assert!(
        validate_status_transition(role, state, false, None, req).is_err(),
        "expected Err for {role:?} + {state:?} + {req:?}"
    );
}

#[test]
fn member_going_upcoming() {
    ok(
        Role::Member,
        DayTimeState::Upcoming,
        Some(AttendanceStatus::Going),
    );
}

#[test]
fn member_not_going_upcoming() {
    ok(
        Role::Member,
        DayTimeState::Upcoming,
        Some(AttendanceStatus::NotGoing),
    );
}

#[test]
fn member_going_started() {
    ok(
        Role::Member,
        DayTimeState::Started,
        Some(AttendanceStatus::Going),
    );
}

#[test]
fn member_going_ended_is_err() {
    err(
        Role::Member,
        DayTimeState::Ended,
        Some(AttendanceStatus::Going),
    );
}

#[test]
fn member_attended_before_end_is_err() {
    err(
        Role::Member,
        DayTimeState::Upcoming,
        Some(AttendanceStatus::Attended),
    );
    err(
        Role::Member,
        DayTimeState::Started,
        Some(AttendanceStatus::Attended),
    );
}

#[test]
fn member_attended_after_end_is_forbidden() {
    let result = validate_status_transition(
        Role::Member,
        DayTimeState::Ended,
        false,
        None,
        Some(AttendanceStatus::Attended),
    );
    assert_eq!(result, Err(StatusTransitionError::Forbidden));
}

#[test]
fn admin_attended_after_end_ok() {
    ok(
        Role::Admin,
        DayTimeState::Ended,
        Some(AttendanceStatus::Attended),
    );
}

#[test]
fn clear_to_no_answer_upcoming_ok() {
    ok(Role::Member, DayTimeState::Upcoming, None);
}

#[test]
fn clear_to_no_answer_ended_is_err() {
    err(Role::Member, DayTimeState::Ended, None);
}

#[test]
fn cancelled_event_always_err() {
    let result = validate_status_transition(
        Role::Admin,
        DayTimeState::Upcoming,
        true, // cancelled
        None,
        Some(AttendanceStatus::Going),
    );
    assert_eq!(result, Err(StatusTransitionError::EventCancelled));
}
