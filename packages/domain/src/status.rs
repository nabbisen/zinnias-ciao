use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Participation status for one member on one event day.
/// `None` is the canonical "No answer" — distinct from every explicit value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceStatus {
    Going,
    NotGoing,
    Attended,
}

/// Time-state of a single event day, computed from server time only (RFC-018).
/// Never derived from client-supplied values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayTimeState {
    Upcoming,
    Started,
    Ended,
}

/// Role of the actor requesting the transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Member,
    Admin,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StatusTransitionError {
    #[error("This event has not started yet")]
    AttendedBeforeEnd,
    #[error("Status changes are not allowed after the event has ended")]
    ChangesAfterEnd,
    #[error("You do not have permission to set this status")]
    Forbidden,
    #[error("This event has been cancelled")]
    EventCancelled,
}

/// Validate a requested status change (RFC-006).
///
/// `current` is `None` when the member has no attendance row yet (No answer).
/// Returns `Ok(())` if the transition is allowed, `Err(reason)` otherwise.
pub fn validate_status_transition(
    role: Role,
    day_time_state: DayTimeState,
    is_event_cancelled: bool,
    current: Option<AttendanceStatus>,
    requested: Option<AttendanceStatus>, // None = clear to No answer
) -> Result<(), StatusTransitionError> {
    if is_event_cancelled {
        return Err(StatusTransitionError::EventCancelled);
    }

    match requested {
        // Clearing to No answer: only allowed before a day ends
        None => match day_time_state {
            DayTimeState::Ended => Err(StatusTransitionError::ChangesAfterEnd),
            _ => Ok(()),
        },

        Some(AttendanceStatus::Going) | Some(AttendanceStatus::NotGoing) => {
            match day_time_state {
                DayTimeState::Ended => Err(StatusTransitionError::ChangesAfterEnd),
                _ => {
                    // Members and admins can set Going/NotGoing before or during a day
                    Ok(())
                }
            }
        }

        Some(AttendanceStatus::Attended) => match day_time_state {
            // Attended only allowed after a day ends (RFC-006)
            DayTimeState::Upcoming | DayTimeState::Started => {
                Err(StatusTransitionError::AttendedBeforeEnd)
            }
            DayTimeState::Ended => {
                // Members cannot set Attended; only admins (RFC-006 / requirements OPD-1)
                if role == Role::Admin {
                    Ok(())
                } else {
                    let _ = current; // not needed for this check
                    Err(StatusTransitionError::Forbidden)
                }
            }
        },
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests live in tests/status.rs (RFC-001: separate test file)
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
