//! Closed audit domain model and required-batch statement builder.
//!
//! The tree remains deliberately non-deployable until every later-package
//! caller has moved off the private compatibility writer.

use crate::crypto::random_token;
use crate::db::now_utc;
use serde_json::{Map, Value, json};
use worker::{
    D1Database, D1PreparedStatement, D1Result, D1Type, Result, console_error, console_log,
};

const MAX_ID_BYTES: usize = 128;
const MAX_REQUEST_ID_BYTES: usize = 96;
const MAX_METADATA_DEPTH: usize = 8;
const MAX_METADATA_NODES: usize = 128;
const MAX_METADATA_BYTES: usize = 2_048;
const MAX_CHANGED_COUNT: u32 = 10_000;
const AUDIT_INSERT_SQL: &str = "INSERT INTO audit_log \
    (id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuditAction {
    CommunityCreated,
    MembershipCreatedFirstAdmin,
    MembershipDisplayNameUpdated,
    InviteCodeGenerated,
    InviteCodeRevoked,
    InviteCodeRedeemed,
    MembershipRelinkCodeCreated,
    MembershipRelinkRedeemed,
    OperatorRecoveryAdminRelinkCreated,
    MembershipRemoved,
    MembershipPromotedToAdmin,
    MembershipDemotedToMember,
    EventCreated,
    EventEdited,
    EventCancelled,
    EventOccurrenceCancelled,
    AttendanceAdminOverride,
    AttendanceAdminSetAttended,
    EventNoteAdminHidden,
    CalendarFeedTokenGenerated,
    CalendarFeedTokenRevoked,
    EventTemplateCreated,
    EventTemplateDeleted,
    CommunityExportAuthorized,
    CalendarMatrixCsvExportRequested,
    SessionLogout,
}

impl AuditAction {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 26] = [
        Self::CommunityCreated,
        Self::MembershipCreatedFirstAdmin,
        Self::MembershipDisplayNameUpdated,
        Self::InviteCodeGenerated,
        Self::InviteCodeRevoked,
        Self::InviteCodeRedeemed,
        Self::MembershipRelinkCodeCreated,
        Self::MembershipRelinkRedeemed,
        Self::OperatorRecoveryAdminRelinkCreated,
        Self::MembershipRemoved,
        Self::MembershipPromotedToAdmin,
        Self::MembershipDemotedToMember,
        Self::EventCreated,
        Self::EventEdited,
        Self::EventCancelled,
        Self::EventOccurrenceCancelled,
        Self::AttendanceAdminOverride,
        Self::AttendanceAdminSetAttended,
        Self::EventNoteAdminHidden,
        Self::CalendarFeedTokenGenerated,
        Self::CalendarFeedTokenRevoked,
        Self::EventTemplateCreated,
        Self::EventTemplateDeleted,
        Self::CommunityExportAuthorized,
        Self::CalendarMatrixCsvExportRequested,
        Self::SessionLogout,
    ];

    pub(crate) const fn canonical(self) -> &'static str {
        match self {
            Self::CommunityCreated => "community.created",
            Self::MembershipCreatedFirstAdmin => "membership.created_first_admin",
            Self::MembershipDisplayNameUpdated => "membership.display_name_updated",
            Self::InviteCodeGenerated => "invite_code.generated",
            Self::InviteCodeRevoked => "invite_code.revoked",
            Self::InviteCodeRedeemed => "invite_code.redeemed",
            Self::MembershipRelinkCodeCreated => "membership.relink_code_created",
            Self::MembershipRelinkRedeemed => "membership.relink_redeemed",
            Self::OperatorRecoveryAdminRelinkCreated => "operator_recovery.admin_relink_created",
            Self::MembershipRemoved => "membership.removed",
            Self::MembershipPromotedToAdmin => "membership.promoted_to_admin",
            Self::MembershipDemotedToMember => "membership.demoted_to_member",
            Self::EventCreated => "event.created",
            Self::EventEdited => "event.edited",
            Self::EventCancelled => "event.cancelled",
            Self::EventOccurrenceCancelled => "event.occurrence_cancelled",
            Self::AttendanceAdminOverride => "attendance.admin_override",
            Self::AttendanceAdminSetAttended => "attendance.admin_set_attended",
            Self::EventNoteAdminHidden => "event_note.admin_hidden",
            Self::CalendarFeedTokenGenerated => "calendar_feed.token_generated",
            Self::CalendarFeedTokenRevoked => "calendar_feed.token_revoked",
            Self::EventTemplateCreated => "event_template.created",
            Self::EventTemplateDeleted => "event_template.deleted",
            Self::CommunityExportAuthorized => "community.export_authorized",
            Self::CalendarMatrixCsvExportRequested => "calendar_matrix_csv.export_requested",
            Self::SessionLogout => "session.logout",
        }
    }

    const fn target_kind(self) -> &'static str {
        match self {
            Self::CommunityCreated | Self::CommunityExportAuthorized => "community",
            Self::MembershipCreatedFirstAdmin
            | Self::MembershipDisplayNameUpdated
            | Self::MembershipRelinkCodeCreated
            | Self::MembershipRelinkRedeemed
            | Self::OperatorRecoveryAdminRelinkCreated
            | Self::MembershipRemoved
            | Self::MembershipPromotedToAdmin
            | Self::MembershipDemotedToMember => "membership",
            Self::InviteCodeGenerated | Self::InviteCodeRevoked | Self::InviteCodeRedeemed => {
                "invite_code"
            }
            Self::EventCreated | Self::EventEdited | Self::EventCancelled => "event",
            Self::EventOccurrenceCancelled => "event_day",
            Self::AttendanceAdminOverride | Self::AttendanceAdminSetAttended => "attendance",
            Self::EventNoteAdminHidden => "event_note",
            Self::CalendarFeedTokenGenerated | Self::CalendarFeedTokenRevoked => "calendar_feed",
            Self::EventTemplateCreated | Self::EventTemplateDeleted => "event_template",
            Self::CalendarMatrixCsvExportRequested => "calendar_matrix_csv",
            Self::SessionLogout => "session",
        }
    }

    #[cfg(test)]
    fn from_canonical(value: &str) -> std::result::Result<Self, AuditBuildError> {
        Self::ALL
            .into_iter()
            .find(|action| action.canonical() == value)
            .ok_or(AuditBuildError::UnknownAction)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum EventCreationMode {
    New,
    CancelledRecreate,
    EventCopy,
}

impl EventCreationMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::CancelledRecreate => "cancelled_recreate",
            Self::EventCopy => "event_copy",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum EventEditScope {
    DetailsOnly,
    SingleDaySchedule,
}

impl EventEditScope {
    const fn as_str(self) -> &'static str {
        match self {
            Self::DetailsOnly => "details_only",
            Self::SingleDaySchedule => "single_day_schedule",
        }
    }
}

pub(crate) enum AuditMetadata {
    None,
    DisplayNameChanged,
    RelinkCorrelation {
        relink_code_id: String,
    },
    OperatorRecovery {
        operator_label: String,
        relink_code_id: String,
    },
    EventCreated {
        creation_mode: EventCreationMode,
        source_event_id: Option<String>,
    },
    EventEdited {
        edit_scope: EventEditScope,
    },
    OccurrenceCancelled {
        series_id: String,
        day_date: String,
    },
    AttendanceOverride {
        changed_count: u32,
    },
    AdminNoteHidden {
        target_membership_id: String,
    },
    MatrixExportRequested {
        month: String,
    },
}

impl AuditMetadata {
    fn to_value(&self) -> Value {
        match self {
            Self::None => json!({}),
            Self::DisplayNameChanged => json!({ "changed_fields": ["display_name"] }),
            Self::RelinkCorrelation { relink_code_id } => {
                json!({ "relink_code_id": relink_code_id })
            }
            Self::OperatorRecovery {
                operator_label,
                relink_code_id,
            } => json!({
                "operator_label": operator_label,
                "relink_code_id": relink_code_id,
            }),
            Self::EventCreated {
                creation_mode,
                source_event_id,
            } => json!({
                "creation_mode": creation_mode.as_str(),
                "source_event_id": source_event_id,
            }),
            Self::EventEdited { edit_scope } => json!({ "edit_scope": edit_scope.as_str() }),
            Self::OccurrenceCancelled {
                series_id,
                day_date,
            } => json!({ "series_id": series_id, "day_date": day_date }),
            Self::AttendanceOverride { changed_count } => {
                json!({ "changed_count": changed_count })
            }
            Self::AdminNoteHidden {
                target_membership_id,
            } => json!({ "target_membership_id": target_membership_id }),
            Self::MatrixExportRequested { month } => json!({ "month": month }),
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum LegacyAuditAction {
    CommunityCreated,
    MembershipCreatedFirstAdmin,
    MembershipDisplayNameUpdated,
    InviteCodeGenerated,
    InviteCodeRevoked,
    InviteCodeRedeemed,
    MembershipRelinkCodeCreated,
    MembershipRelinkRedeemed,
    OperatorRecoveryAdminRelinkCreated,
    MembershipRemoved,
    MembershipPromotedToAdmin,
    MembershipDemotedToMember,
    EventCreated,
    EventEdited,
    EventCancelled,
    EventOccurrenceCancelled,
    AttendanceAdminOverride,
    AttendanceAdminSetAttended,
    EventNoteAdminHidden,
    CalendarFeedTokenGenerated,
    CalendarFeedTokenRevoked,
    EventTemplateCreated,
    EventTemplateDeleted,
    CommunityExportAuthorized,
    CalendarMatrixCsvExportRequested,
    SessionLogout,
}

impl From<LegacyAuditAction> for AuditAction {
    fn from(value: LegacyAuditAction) -> Self {
        match value {
            LegacyAuditAction::CommunityCreated => Self::CommunityCreated,
            LegacyAuditAction::MembershipCreatedFirstAdmin => Self::MembershipCreatedFirstAdmin,
            LegacyAuditAction::MembershipDisplayNameUpdated => Self::MembershipDisplayNameUpdated,
            LegacyAuditAction::InviteCodeGenerated => Self::InviteCodeGenerated,
            LegacyAuditAction::InviteCodeRevoked => Self::InviteCodeRevoked,
            LegacyAuditAction::InviteCodeRedeemed => Self::InviteCodeRedeemed,
            LegacyAuditAction::MembershipRelinkCodeCreated => Self::MembershipRelinkCodeCreated,
            LegacyAuditAction::MembershipRelinkRedeemed => Self::MembershipRelinkRedeemed,
            LegacyAuditAction::OperatorRecoveryAdminRelinkCreated => {
                Self::OperatorRecoveryAdminRelinkCreated
            }
            LegacyAuditAction::MembershipRemoved => Self::MembershipRemoved,
            LegacyAuditAction::MembershipPromotedToAdmin => Self::MembershipPromotedToAdmin,
            LegacyAuditAction::MembershipDemotedToMember => Self::MembershipDemotedToMember,
            LegacyAuditAction::EventCreated => Self::EventCreated,
            LegacyAuditAction::EventEdited => Self::EventEdited,
            LegacyAuditAction::EventCancelled => Self::EventCancelled,
            LegacyAuditAction::EventOccurrenceCancelled => Self::EventOccurrenceCancelled,
            LegacyAuditAction::AttendanceAdminOverride => Self::AttendanceAdminOverride,
            LegacyAuditAction::AttendanceAdminSetAttended => Self::AttendanceAdminSetAttended,
            LegacyAuditAction::EventNoteAdminHidden => Self::EventNoteAdminHidden,
            LegacyAuditAction::CalendarFeedTokenGenerated => Self::CalendarFeedTokenGenerated,
            LegacyAuditAction::CalendarFeedTokenRevoked => Self::CalendarFeedTokenRevoked,
            LegacyAuditAction::EventTemplateCreated => Self::EventTemplateCreated,
            LegacyAuditAction::EventTemplateDeleted => Self::EventTemplateDeleted,
            LegacyAuditAction::CommunityExportAuthorized => Self::CommunityExportAuthorized,
            LegacyAuditAction::CalendarMatrixCsvExportRequested => {
                Self::CalendarMatrixCsvExportRequested
            }
            LegacyAuditAction::SessionLogout => Self::SessionLogout,
        }
    }
}

#[allow(dead_code)]
pub(crate) enum LegacyAuditMetadata {
    None,
    DisplayNameChanged,
    RelinkCorrelation {
        relink_code_id: String,
    },
    OperatorRecovery {
        operator_label: String,
        relink_code_id: String,
    },
    EventCreated {
        creation_mode: EventCreationMode,
        source_event_id: Option<String>,
    },
    EventEdited {
        edit_scope: EventEditScope,
    },
    OccurrenceCancelled {
        series_id: String,
        day_date: String,
    },
    AttendanceOverride {
        changed_count: u32,
    },
    AdminNoteHidden {
        target_membership_id: String,
    },
    MatrixExportRequested {
        month: String,
    },
}

impl From<LegacyAuditMetadata> for AuditMetadata {
    fn from(value: LegacyAuditMetadata) -> Self {
        match value {
            LegacyAuditMetadata::None => Self::None,
            LegacyAuditMetadata::DisplayNameChanged => Self::DisplayNameChanged,
            LegacyAuditMetadata::RelinkCorrelation { relink_code_id } => {
                Self::RelinkCorrelation { relink_code_id }
            }
            LegacyAuditMetadata::OperatorRecovery {
                operator_label,
                relink_code_id,
            } => Self::OperatorRecovery {
                operator_label,
                relink_code_id,
            },
            LegacyAuditMetadata::EventCreated {
                creation_mode,
                source_event_id,
            } => Self::EventCreated {
                creation_mode,
                source_event_id,
            },
            LegacyAuditMetadata::EventEdited { edit_scope } => Self::EventEdited { edit_scope },
            LegacyAuditMetadata::OccurrenceCancelled {
                series_id,
                day_date,
            } => Self::OccurrenceCancelled {
                series_id,
                day_date,
            },
            LegacyAuditMetadata::AttendanceOverride { changed_count } => {
                Self::AttendanceOverride { changed_count }
            }
            LegacyAuditMetadata::AdminNoteHidden {
                target_membership_id,
            } => Self::AdminNoteHidden {
                target_membership_id,
            },
            LegacyAuditMetadata::MatrixExportRequested { month } => {
                Self::MatrixExportRequested { month }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuditBuildError {
    #[cfg(test)]
    UnknownAction,
    IncompatibleMetadata,
    InvalidIdentifier,
    InvalidRequestId,
    InvalidMetadataField,
    MetadataRoot,
    MetadataDepth,
    MetadataNodes,
    MetadataBytes,
}

impl AuditBuildError {
    const fn category(self) -> &'static str {
        match self {
            #[cfg(test)]
            Self::UnknownAction => "incompatible",
            Self::IncompatibleMetadata => "incompatible",
            Self::InvalidIdentifier | Self::InvalidRequestId | Self::InvalidMetadataField => {
                "invalid_field"
            }
            Self::MetadataRoot
            | Self::MetadataDepth
            | Self::MetadataNodes
            | Self::MetadataBytes => "metadata_bound",
        }
    }
}

pub(crate) struct AuditRecord {
    id: String,
    request_id: String,
    community_id: Option<String>,
    actor_membership_id: Option<String>,
    target_id: Option<String>,
    action: AuditAction,
    metadata_json: String,
    created_at: String,
}

impl AuditRecord {
    pub(crate) fn new(
        request_id: &str,
        community_id: Option<&str>,
        actor_membership_id: Option<&str>,
        target_id: Option<&str>,
        action: AuditAction,
        metadata: AuditMetadata,
    ) -> std::result::Result<Self, AuditBuildError> {
        Self::build(
            format!("aud_{}", &random_token()[..24]),
            now_utc(),
            request_id,
            community_id,
            actor_membership_id,
            target_id,
            action,
            metadata,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        id: String,
        created_at: String,
        request_id: &str,
        community_id: Option<&str>,
        actor_membership_id: Option<&str>,
        target_id: Option<&str>,
        action: AuditAction,
        metadata: AuditMetadata,
    ) -> std::result::Result<Self, AuditBuildError> {
        validate_request_id(request_id)?;
        for id in [community_id, actor_membership_id, target_id]
            .into_iter()
            .flatten()
        {
            validate_identifier(id)?;
        }
        validate_context(action, community_id, actor_membership_id, target_id)?;
        validate_pairing(action, &metadata)?;
        validate_metadata_fields(&metadata)?;
        let metadata_json = sanitize_and_serialize(metadata.to_value())?;
        Ok(Self {
            id,
            request_id: request_id.to_owned(),
            community_id: community_id.map(str::to_owned),
            actor_membership_id: actor_membership_id.map(str::to_owned),
            target_id: target_id.map(str::to_owned),
            action,
            metadata_json,
            created_at,
        })
    }

    pub(crate) fn statement(&self, db: &D1Database) -> Result<worker::D1PreparedStatement> {
        self.statement_with_suffix(db, "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")
    }

    /// Insert this record only when the immediately preceding statement changed
    /// exactly one row. Callers must keep the guarded mutation and this
    /// statement adjacent in the same D1 batch.
    pub(crate) fn statement_after_one_change(
        &self,
        db: &D1Database,
    ) -> Result<D1PreparedStatement> {
        self.statement_with_suffix(
            db,
            "SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9 WHERE changes() = 1",
        )
    }

    /// Insert this record when the adjacent set-based mutation changed at
    /// least one row, up to the caller's explicit domain bound.
    pub(crate) fn statement_after_bounded_changes(
        &self,
        db: &D1Database,
        max_changes: u32,
    ) -> Result<D1PreparedStatement> {
        let suffix = format!(
            "SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9 \
             WHERE changes() BETWEEN 1 AND {max_changes}"
        );
        self.statement_with_suffix(db, &suffix)
    }

    /// Attendance override counts must come from the set-based database
    /// mutation itself, not from a racy application-side estimate.
    fn attendance_statement_after_bounded_changes(
        &self,
        db: &D1Database,
        max_changes: u32,
    ) -> Result<D1PreparedStatement> {
        if self.action != AuditAction::AttendanceAdminOverride {
            return Err(worker::Error::RustError(
                "dynamic attendance metadata used with incompatible action".to_owned(),
            ));
        }
        let community_id = self
            .community_id
            .as_deref()
            .map(D1Type::Text)
            .unwrap_or(D1Type::Null);
        let actor_membership_id = self
            .actor_membership_id
            .as_deref()
            .map(D1Type::Text)
            .unwrap_or(D1Type::Null);
        let target_id = self
            .target_id
            .as_deref()
            .map(D1Type::Text)
            .unwrap_or(D1Type::Null);
        let values = [
            D1Type::Text(self.id.as_str()),
            D1Type::Text(self.request_id.as_str()),
            community_id,
            actor_membership_id,
            D1Type::Text(self.action.target_kind()),
            target_id,
            D1Type::Text(self.action.canonical()),
            D1Type::Text(self.created_at.as_str()),
        ];
        db.prepare(format!(
            "{AUDIT_INSERT_SQL} \
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, \
                    json_object('changed_count', changes()), ?8 \
             WHERE changes() BETWEEN 1 AND {max_changes}"
        ))
        .bind_refs(&values)
    }

    fn statement_with_suffix(
        &self,
        db: &D1Database,
        values_sql: &str,
    ) -> Result<D1PreparedStatement> {
        let community_id = self
            .community_id
            .as_deref()
            .map(D1Type::Text)
            .unwrap_or(D1Type::Null);
        let actor_membership_id = self
            .actor_membership_id
            .as_deref()
            .map(D1Type::Text)
            .unwrap_or(D1Type::Null);
        let target_id = self
            .target_id
            .as_deref()
            .map(D1Type::Text)
            .unwrap_or(D1Type::Null);
        let values = [
            D1Type::Text(self.id.as_str()),
            D1Type::Text(self.request_id.as_str()),
            community_id,
            actor_membership_id,
            D1Type::Text(self.action.target_kind()),
            target_id,
            D1Type::Text(self.action.canonical()),
            D1Type::Text(self.metadata_json.as_str()),
            D1Type::Text(self.created_at.as_str()),
        ];
        db.prepare(format!("{AUDIT_INSERT_SQL} {values_sql}"))
            .bind_refs(&values)
    }

    fn success_event(&self) -> String {
        format_event(&self.request_id, self.action, AuditOutcome::Success)
    }

    pub(crate) fn log_success(&self) {
        console_log!("{}", self.success_event());
    }
}

pub(crate) fn required_record(
    request_id: &str,
    community_id: Option<&str>,
    actor_membership_id: Option<&str>,
    target_id: Option<&str>,
    action: AuditAction,
    metadata: AuditMetadata,
) -> Result<AuditRecord> {
    AuditRecord::new(
        request_id,
        community_id,
        actor_membership_id,
        target_id,
        action,
        metadata,
    )
    .map_err(|error| {
        worker::Error::RustError(format!("audit construction rejected: {}", error.category()))
    })
}

pub(crate) async fn execute_required(
    db: &D1Database,
    mutation: D1PreparedStatement,
    audit: &AuditRecord,
) -> Result<bool> {
    let results = db
        .batch(vec![mutation, audit.statement_after_one_change(db)?])
        .await?;
    let mutation_changes = result_changes(&results, 0);
    let audit_changes = result_changes(&results, 1);
    match (mutation_changes, audit_changes) {
        (0, 0) => Ok(false),
        (1, 1) => {
            audit.log_success();
            Ok(true)
        }
        _ => Err(worker::Error::RustError(format!(
            "required audit cardinality mismatch: mutation={mutation_changes} audit={audit_changes}"
        ))),
    }
}

pub(crate) async fn execute_required_bounded(
    db: &D1Database,
    mutation: D1PreparedStatement,
    audit: &AuditRecord,
    max_changes: u32,
) -> Result<usize> {
    if max_changes == 0 || max_changes > MAX_CHANGED_COUNT {
        return Err(worker::Error::RustError(
            "required audit change bound rejected".to_owned(),
        ));
    }
    let results = db
        .batch(vec![
            mutation,
            audit.statement_after_bounded_changes(db, max_changes)?,
        ])
        .await?;
    let mutation_changes = result_changes(&results, 0);
    let audit_changes = result_changes(&results, 1);
    if mutation_changes == 0 && audit_changes == 0 {
        return Ok(0);
    }
    if mutation_changes <= max_changes as usize && audit_changes == 1 {
        audit.log_success();
        return Ok(mutation_changes);
    }
    Err(worker::Error::RustError(format!(
        "required bounded audit cardinality mismatch: mutation={mutation_changes} audit={audit_changes}"
    )))
}

pub(crate) async fn execute_required_attendance_override(
    db: &D1Database,
    mutation: D1PreparedStatement,
    audit: &AuditRecord,
    max_changes: u32,
) -> Result<usize> {
    if max_changes == 0 || max_changes > MAX_CHANGED_COUNT {
        return Err(worker::Error::RustError(
            "attendance override change bound rejected".to_owned(),
        ));
    }
    let results = db
        .batch(vec![
            mutation,
            audit.attendance_statement_after_bounded_changes(db, max_changes)?,
        ])
        .await?;
    let mutation_changes = result_changes(&results, 0);
    let audit_changes = result_changes(&results, 1);
    if mutation_changes == 0 && audit_changes == 0 {
        return Ok(0);
    }
    if mutation_changes <= max_changes as usize && audit_changes == 1 {
        audit.log_success();
        return Ok(mutation_changes);
    }
    Err(worker::Error::RustError(format!(
        "attendance audit cardinality mismatch: mutation={mutation_changes} audit={audit_changes}"
    )))
}

pub(crate) async fn execute_required_batch(
    db: &D1Database,
    mut business: Vec<D1PreparedStatement>,
    audits: &[AuditRecord],
) -> Result<Vec<D1Result>> {
    for audit in audits {
        business.push(audit.statement(db)?);
    }
    let results = db.batch(business).await?;
    for audit in audits {
        audit.log_success();
    }
    Ok(results)
}

/// Execute a bounded multi-statement business transition whose final
/// statement is the success witness. The audit remains adjacent to that
/// witness and is omitted when the whole guarded chain is a no-op.
pub(crate) async fn execute_required_tail(
    db: &D1Database,
    mut business: Vec<D1PreparedStatement>,
    audit: &AuditRecord,
) -> Result<bool> {
    if business.is_empty() {
        return Err(worker::Error::RustError(
            "required audit business batch is empty".to_owned(),
        ));
    }
    let tail_index = business.len() - 1;
    business.push(audit.statement_after_one_change(db)?);
    let results = db.batch(business).await?;
    let tail_changes = result_changes(&results, tail_index);
    let audit_changes = result_changes(&results, tail_index + 1);
    match (tail_changes, audit_changes) {
        (0, 0) => Ok(false),
        (1, 1) => {
            audit.log_success();
            Ok(true)
        }
        _ => Err(worker::Error::RustError(format!(
            "required tail audit cardinality mismatch: mutation={tail_changes} audit={audit_changes}"
        ))),
    }
}

/// Execute a one-winner transition using the accepted Package 0 assertion
/// primitive. The private operation ID exists only in the assertion table and
/// is deleted in the same transaction after the required audit is inserted.
pub(crate) async fn execute_asserted_required(
    db: &D1Database,
    claim: D1PreparedStatement,
    required: Vec<D1PreparedStatement>,
    optional: Vec<D1PreparedStatement>,
    audit: &AuditRecord,
) -> Result<Vec<D1Result>> {
    let operation_id = format!("ast_{}", &random_token()[..22]);
    let assertion = db
        .prepare(
            "INSERT INTO audit_change_assertions (operation_id, changed_count) \
             VALUES (?1, changes())",
        )
        .bind(&[operation_id.as_str().into()])?;
    let cleanup = db
        .prepare("DELETE FROM audit_change_assertions WHERE operation_id=?1")
        .bind(&[operation_id.as_str().into()])?;

    let mut statements = Vec::with_capacity(required.len() * 2 + optional.len() + 4);
    statements.push(claim);
    statements.push(assertion);
    let mut verification_indices = Vec::with_capacity(required.len());
    for statement in required {
        statements.push(statement);
        verification_indices.push(statements.len());
        statements.push(
            db.prepare(
                "UPDATE audit_change_assertions SET changed_count=changes() \
                 WHERE operation_id=?1",
            )
            .bind(&[operation_id.as_str().into()])?,
        );
    }
    statements.extend(optional);
    let audit_index = statements.len();
    statements.push(audit.statement(db)?);
    let cleanup_index = statements.len();
    statements.push(cleanup);

    let results = db.batch(statements).await?;
    let claim_changes = result_changes(&results, 0);
    let assertion_changes = result_changes(&results, 1);
    let audit_changes = result_changes(&results, audit_index);
    let cleanup_changes = result_changes(&results, cleanup_index);
    let required_verified = verification_indices
        .iter()
        .all(|index| result_changes(&results, *index) == 1);
    if (
        claim_changes,
        assertion_changes,
        audit_changes,
        cleanup_changes,
    ) != (1, 1, 1, 1)
        || !required_verified
    {
        return Err(worker::Error::RustError(format!(
            "asserted audit cardinality mismatch: claim={claim_changes} assertion={assertion_changes} audit={audit_changes} cleanup={cleanup_changes}"
        )));
    }
    audit.log_success();
    Ok(results)
}

/// Persist Class B authorization evidence before any protected response is
/// returned. Construction and storage failures emit only bounded operational
/// fields; callers must translate the error into a disclosure-free 503.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn write_pre_disclosure(
    db: &D1Database,
    request_id: &str,
    community_id: &str,
    actor_membership_id: &str,
    target_id: &str,
    action: AuditAction,
    metadata: AuditMetadata,
) -> Result<()> {
    if !matches!(
        action,
        AuditAction::CommunityExportAuthorized | AuditAction::CalendarMatrixCsvExportRequested
    ) {
        log_failure(
            request_id,
            action,
            AuditFailureEvent::PreDisclosure,
            AuditFailureCategory::Construction,
        );
        return Err(worker::Error::RustError(
            "non-Class-B pre-disclosure audit rejected".to_owned(),
        ));
    }
    let record = match required_record(
        request_id,
        Some(community_id),
        Some(actor_membership_id),
        Some(target_id),
        action,
        metadata,
    ) {
        Ok(record) => record,
        Err(error) => {
            log_failure(
                request_id,
                action,
                AuditFailureEvent::PreDisclosure,
                AuditFailureCategory::Construction,
            );
            return Err(error);
        }
    };
    let statement = match record.statement(db) {
        Ok(statement) => statement,
        Err(error) => {
            log_failure(
                request_id,
                action,
                AuditFailureEvent::PreDisclosure,
                AuditFailureCategory::Construction,
            );
            return Err(error);
        }
    };
    if let Err(error) = statement.run().await {
        log_failure(
            request_id,
            action,
            AuditFailureEvent::PreDisclosure,
            AuditFailureCategory::Storage,
        );
        return Err(error);
    }
    record.log_success();
    Ok(())
}

/// Logout is the sole Class C safety-first exception. This helper deliberately
/// accepts no subject or session identifier and owns its bounded failure
/// incident so the caller can always continue to cookie clearing.
pub(crate) async fn write_logout_secondary(db: &D1Database, request_id: &str) {
    let action = AuditAction::SessionLogout;
    let record = match required_record(request_id, None, None, None, action, AuditMetadata::None) {
        Ok(record) => record,
        Err(_) => {
            log_failure(
                request_id,
                action,
                AuditFailureEvent::SecondaryWrite,
                AuditFailureCategory::Construction,
            );
            return;
        }
    };
    let statement = match record.statement(db) {
        Ok(statement) => statement,
        Err(_) => {
            log_failure(
                request_id,
                action,
                AuditFailureEvent::SecondaryWrite,
                AuditFailureCategory::Construction,
            );
            return;
        }
    };
    if statement.run().await.is_err() {
        log_failure(
            request_id,
            action,
            AuditFailureEvent::SecondaryWrite,
            AuditFailureCategory::Storage,
        );
        return;
    }
    record.log_success();
}

pub(crate) fn result_changes(results: &[D1Result], index: usize) -> usize {
    results
        .get(index)
        .and_then(|result| result.meta().ok().flatten())
        .and_then(|meta| meta.changes)
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub(crate) async fn write_legacy(
    db: &D1Database,
    request_id: &str,
    community_id: Option<&str>,
    actor_membership_id: Option<&str>,
    target_id: Option<&str>,
    action: LegacyAuditAction,
    metadata: LegacyAuditMetadata,
) -> Result<()> {
    let action = AuditAction::from(action);
    let record = required_record(
        request_id,
        community_id,
        actor_membership_id,
        target_id,
        action,
        metadata.into(),
    )?;
    record.statement(db)?.run().await?;
    record.log_success();
    Ok(())
}

fn validate_pairing(
    action: AuditAction,
    metadata: &AuditMetadata,
) -> std::result::Result<(), AuditBuildError> {
    let valid = matches!(
        (action, metadata),
        (
            AuditAction::MembershipDisplayNameUpdated,
            AuditMetadata::DisplayNameChanged
        ) | (
            AuditAction::MembershipRelinkCodeCreated,
            AuditMetadata::RelinkCorrelation { .. }
        ) | (
            AuditAction::MembershipRelinkRedeemed,
            AuditMetadata::RelinkCorrelation { .. }
        ) | (
            AuditAction::OperatorRecoveryAdminRelinkCreated,
            AuditMetadata::OperatorRecovery { .. }
        ) | (
            AuditAction::EventCreated,
            AuditMetadata::EventCreated { .. }
        ) | (AuditAction::EventEdited, AuditMetadata::EventEdited { .. })
            | (
                AuditAction::EventOccurrenceCancelled,
                AuditMetadata::OccurrenceCancelled { .. }
            )
            | (
                AuditAction::AttendanceAdminOverride,
                AuditMetadata::AttendanceOverride { .. }
            )
            | (
                AuditAction::EventNoteAdminHidden,
                AuditMetadata::AdminNoteHidden { .. }
            )
            | (
                AuditAction::CalendarMatrixCsvExportRequested,
                AuditMetadata::MatrixExportRequested { .. }
            )
    ) || matches!(metadata, AuditMetadata::None)
        && matches!(
            action,
            AuditAction::CommunityCreated
                | AuditAction::MembershipCreatedFirstAdmin
                | AuditAction::InviteCodeGenerated
                | AuditAction::InviteCodeRevoked
                | AuditAction::InviteCodeRedeemed
                | AuditAction::MembershipRemoved
                | AuditAction::MembershipPromotedToAdmin
                | AuditAction::MembershipDemotedToMember
                | AuditAction::EventCancelled
                | AuditAction::AttendanceAdminSetAttended
                | AuditAction::CalendarFeedTokenGenerated
                | AuditAction::CalendarFeedTokenRevoked
                | AuditAction::EventTemplateCreated
                | AuditAction::EventTemplateDeleted
                | AuditAction::CommunityExportAuthorized
                | AuditAction::SessionLogout
        );
    if valid {
        Ok(())
    } else {
        Err(AuditBuildError::IncompatibleMetadata)
    }
}

fn validate_context(
    action: AuditAction,
    community_id: Option<&str>,
    actor_membership_id: Option<&str>,
    target_id: Option<&str>,
) -> std::result::Result<(), AuditBuildError> {
    let valid = match action {
        AuditAction::SessionLogout => {
            community_id.is_none() && actor_membership_id.is_none() && target_id.is_none()
        }
        AuditAction::CalendarFeedTokenGenerated | AuditAction::CalendarFeedTokenRevoked => {
            community_id.is_some() && actor_membership_id.is_some() && target_id.is_none()
        }
        _ => community_id.is_some() && actor_membership_id.is_some() && target_id.is_some(),
    };
    if valid {
        Ok(())
    } else {
        Err(AuditBuildError::InvalidIdentifier)
    }
}

fn validate_metadata_fields(metadata: &AuditMetadata) -> std::result::Result<(), AuditBuildError> {
    match metadata {
        AuditMetadata::RelinkCorrelation { relink_code_id } => validate_identifier(relink_code_id),
        AuditMetadata::OperatorRecovery {
            operator_label,
            relink_code_id,
        } => {
            validate_operator_label(operator_label)?;
            validate_identifier(relink_code_id)
        }
        AuditMetadata::EventCreated {
            creation_mode,
            source_event_id,
        } => {
            if matches!(creation_mode, EventCreationMode::New) != source_event_id.is_none() {
                return Err(AuditBuildError::InvalidMetadataField);
            }
            if let Some(id) = source_event_id {
                validate_identifier(id)?;
            }
            Ok(())
        }
        AuditMetadata::OccurrenceCancelled {
            series_id,
            day_date,
        } => {
            validate_identifier(series_id)?;
            validate_day(day_date)
        }
        AuditMetadata::AttendanceOverride { changed_count } => {
            if *changed_count <= MAX_CHANGED_COUNT {
                Ok(())
            } else {
                Err(AuditBuildError::InvalidMetadataField)
            }
        }
        AuditMetadata::AdminNoteHidden {
            target_membership_id,
        } => validate_identifier(target_membership_id),
        AuditMetadata::MatrixExportRequested { month } => validate_month(month),
        AuditMetadata::None
        | AuditMetadata::DisplayNameChanged
        | AuditMetadata::EventEdited { .. } => Ok(()),
    }
}

fn validate_identifier(value: &str) -> std::result::Result<(), AuditBuildError> {
    if !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        Ok(())
    } else {
        Err(AuditBuildError::InvalidIdentifier)
    }
}

fn validate_request_id(value: &str) -> std::result::Result<(), AuditBuildError> {
    if !value.is_empty()
        && value.len() <= MAX_REQUEST_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        Ok(())
    } else {
        Err(AuditBuildError::InvalidRequestId)
    }
}

fn validate_operator_label(value: &str) -> std::result::Result<(), AuditBuildError> {
    let bytes = value.as_bytes();
    if (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(byte))
    {
        Ok(())
    } else {
        Err(AuditBuildError::InvalidMetadataField)
    }
}

fn validate_day(value: &str) -> std::result::Result<(), AuditBuildError> {
    let bytes = value.as_bytes();
    let shape_valid = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    let valid_date = shape_valid
        && value[0..4]
            .parse::<i32>()
            .ok()
            .zip(value[5..7].parse::<u8>().ok())
            .zip(value[8..10].parse::<u8>().ok())
            .and_then(|((year, month), day)| {
                time::Month::try_from(month)
                    .ok()
                    .and_then(|month| time::Date::from_calendar_date(year, month, day).ok())
            })
            .is_some();
    if valid_date {
        Ok(())
    } else {
        Err(AuditBuildError::InvalidMetadataField)
    }
}

fn validate_month(value: &str) -> std::result::Result<(), AuditBuildError> {
    let bytes = value.as_bytes();
    if bytes.len() == 7
        && bytes[4] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || byte.is_ascii_digit())
        && (1..=12).contains(&value[5..].parse::<u8>().unwrap_or(0))
    {
        Ok(())
    } else {
        Err(AuditBuildError::InvalidMetadataField)
    }
}

fn sanitize_and_serialize(mut value: Value) -> std::result::Result<String, AuditBuildError> {
    if !value.is_object() {
        return Err(AuditBuildError::MetadataRoot);
    }
    let mut nodes = 0;
    sanitize_value(&mut value, 1, &mut nodes)?;
    let serialized = serde_json::to_string(&value).map_err(|_| AuditBuildError::MetadataBytes)?;
    if serialized.len() > MAX_METADATA_BYTES {
        return Err(AuditBuildError::MetadataBytes);
    }
    Ok(serialized)
}

fn sanitize_value(
    value: &mut Value,
    depth: usize,
    nodes: &mut usize,
) -> std::result::Result<(), AuditBuildError> {
    if depth > MAX_METADATA_DEPTH {
        return Err(AuditBuildError::MetadataDepth);
    }
    *nodes += 1;
    if *nodes > MAX_METADATA_NODES {
        return Err(AuditBuildError::MetadataNodes);
    }
    match value {
        Value::Object(object) => sanitize_object(object, depth, nodes),
        Value::Array(values) => {
            for child in values {
                sanitize_value(child, depth + 1, nodes)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn sanitize_object(
    object: &mut Map<String, Value>,
    depth: usize,
    nodes: &mut usize,
) -> std::result::Result<(), AuditBuildError> {
    for value in object.values_mut() {
        sanitize_value(value, depth + 1, nodes)?;
    }
    object.retain(|key, _| !forbidden_key(key));
    Ok(())
}

fn forbidden_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    const EXACT: &[&str] = &[
        "password",
        "token",
        "secret",
        "code",
        "hmac",
        "session",
        "note",
        "memo",
        "body",
        "pepper",
        "cookie",
        "authorization",
        "content",
        "description",
        "display_name",
        "email",
        "phone",
    ];
    if key == "relink_code_id" {
        return false;
    }
    EXACT.contains(&key.as_str())
        || key.contains("code")
        || key.contains("hmac")
        || key.contains("session")
        || key.contains("memo")
        || [
            "_token",
            "_secret",
            "_hmac",
            "_hash",
            "_password",
            "_cookie",
        ]
        .iter()
        .any(|suffix| key.ends_with(suffix))
        || key.starts_with("session_")
        || key.starts_with("authorization_")
}

#[derive(Clone, Copy)]
enum AuditOutcome {
    Success,
}

#[derive(Clone, Copy)]
enum AuditFailureEvent {
    PreDisclosure,
    SecondaryWrite,
}

impl AuditFailureEvent {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PreDisclosure => "audit.pre_disclosure_failed",
            Self::SecondaryWrite => "audit.secondary_write_failed",
        }
    }

    const fn route_class(self) -> &'static str {
        match self {
            Self::PreDisclosure => "class_b",
            Self::SecondaryWrite => "class_c",
        }
    }
}

#[derive(Clone, Copy)]
enum AuditFailureCategory {
    Construction,
    Storage,
}

impl AuditFailureCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Construction => "construction",
            Self::Storage => "storage",
        }
    }
}

impl AuditOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
        }
    }
}

fn format_event(request_id: &str, action: AuditAction, outcome: AuditOutcome) -> String {
    format!(
        "event=audit.write request_id={} action={} outcome={}",
        request_id,
        action.canonical(),
        outcome.as_str()
    )
}

fn format_failure_event(
    request_id: &str,
    action: AuditAction,
    event: AuditFailureEvent,
    category: AuditFailureCategory,
) -> String {
    format!(
        "event={} request_id={} action={} failure_category={} route_class={}",
        event.as_str(),
        request_id,
        action.canonical(),
        category.as_str(),
        event.route_class(),
    )
}

fn log_failure(
    request_id: &str,
    action: AuditAction,
    event: AuditFailureEvent,
    category: AuditFailureCategory,
) {
    console_error!(
        "{}",
        format_failure_event(request_id, action, event, category)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        action: AuditAction,
        metadata: AuditMetadata,
    ) -> Result<AuditRecord, AuditBuildError> {
        let (community_id, actor_membership_id, target_id) = match action {
            AuditAction::SessionLogout => (None, None, None),
            AuditAction::CalendarFeedTokenGenerated | AuditAction::CalendarFeedTokenRevoked => {
                (Some("community_1"), Some("membership_1"), None)
            }
            _ => (Some("community_1"), Some("membership_1"), Some("target_1")),
        };
        AuditRecord::build(
            "aud_0123456789abcdef012345".into(),
            "2026-07-15T00:00:00Z".into(),
            "0123456789abcdef",
            community_id,
            actor_membership_id,
            target_id,
            action,
            metadata,
        )
    }

    #[test]
    fn every_action_has_one_namespaced_canonical_value() {
        let values: std::collections::BTreeSet<_> = AuditAction::ALL
            .into_iter()
            .map(AuditAction::canonical)
            .collect();
        assert_eq!(values.len(), 26);
        assert!(values.iter().all(|value| value.contains('.')));
        for value in values {
            assert_eq!(
                AuditAction::from_canonical(value).unwrap().canonical(),
                value
            );
        }
        assert!(matches!(
            AuditAction::from_canonical("membership.unknown"),
            Err(AuditBuildError::UnknownAction)
        ));
    }

    #[test]
    fn every_action_accepts_only_its_metadata_pairing() {
        let none_actions = [
            AuditAction::CommunityCreated,
            AuditAction::MembershipCreatedFirstAdmin,
            AuditAction::InviteCodeGenerated,
            AuditAction::InviteCodeRevoked,
            AuditAction::InviteCodeRedeemed,
            AuditAction::MembershipRemoved,
            AuditAction::MembershipPromotedToAdmin,
            AuditAction::MembershipDemotedToMember,
            AuditAction::EventCancelled,
            AuditAction::AttendanceAdminSetAttended,
            AuditAction::CalendarFeedTokenGenerated,
            AuditAction::CalendarFeedTokenRevoked,
            AuditAction::EventTemplateCreated,
            AuditAction::EventTemplateDeleted,
            AuditAction::CommunityExportAuthorized,
            AuditAction::SessionLogout,
        ];
        for action in none_actions {
            assert!(record(action, AuditMetadata::None).is_ok());
            assert!(matches!(
                record(action, AuditMetadata::DisplayNameChanged),
                Err(AuditBuildError::IncompatibleMetadata)
            ));
        }
        let typed = [
            record(
                AuditAction::MembershipDisplayNameUpdated,
                AuditMetadata::DisplayNameChanged,
            ),
            record(
                AuditAction::MembershipRelinkCodeCreated,
                AuditMetadata::RelinkCorrelation {
                    relink_code_id: "relink_1".into(),
                },
            ),
            record(
                AuditAction::MembershipRelinkRedeemed,
                AuditMetadata::RelinkCorrelation {
                    relink_code_id: "relink_1".into(),
                },
            ),
            record(
                AuditAction::OperatorRecoveryAdminRelinkCreated,
                AuditMetadata::OperatorRecovery {
                    operator_label: "operator.primary".into(),
                    relink_code_id: "relink_1".into(),
                },
            ),
            record(
                AuditAction::EventCreated,
                AuditMetadata::EventCreated {
                    creation_mode: EventCreationMode::New,
                    source_event_id: None,
                },
            ),
            record(
                AuditAction::EventEdited,
                AuditMetadata::EventEdited {
                    edit_scope: EventEditScope::DetailsOnly,
                },
            ),
            record(
                AuditAction::EventOccurrenceCancelled,
                AuditMetadata::OccurrenceCancelled {
                    series_id: "series_1".into(),
                    day_date: "2026-07-15".into(),
                },
            ),
            record(
                AuditAction::AttendanceAdminOverride,
                AuditMetadata::AttendanceOverride { changed_count: 1 },
            ),
            record(
                AuditAction::EventNoteAdminHidden,
                AuditMetadata::AdminNoteHidden {
                    target_membership_id: "membership_2".into(),
                },
            ),
            record(
                AuditAction::CalendarMatrixCsvExportRequested,
                AuditMetadata::MatrixExportRequested {
                    month: "2026-07".into(),
                },
            ),
        ];
        assert!(typed.into_iter().all(|result| result.is_ok()));
        assert!(matches!(
            record(AuditAction::EventEdited, AuditMetadata::None),
            Err(AuditBuildError::IncompatibleMetadata)
        ));
    }

    #[test]
    fn metadata_fields_enforce_all_boundaries() {
        assert!(validate_identifier("a").is_ok());
        assert!(validate_identifier(&"a".repeat(MAX_ID_BYTES)).is_ok());
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier(&"a".repeat(MAX_ID_BYTES + 1)).is_err());
        assert!(validate_identifier("unsafe/value").is_err());
        assert!(validate_request_id("r").is_ok());
        assert!(validate_request_id(&"r".repeat(MAX_REQUEST_ID_BYTES)).is_ok());
        assert!(validate_request_id(&"r".repeat(MAX_REQUEST_ID_BYTES + 1)).is_err());
        assert!(validate_request_id("unsafe request").is_err());
        assert!(validate_operator_label("a").is_ok());
        assert!(validate_operator_label("a-1.primary").is_ok());
        assert!(validate_operator_label("A").is_err());
        assert!(validate_operator_label(&"a".repeat(33)).is_err());
        assert!(validate_day("2026-07-15").is_ok());
        assert!(validate_day("2024-02-29").is_ok());
        assert!(validate_day("2026-02-29").is_err());
        assert!(validate_day("2026-7-15").is_err());
        assert!(validate_month("2026-01").is_ok());
        assert!(validate_month("2026-12").is_ok());
        assert!(validate_month("2026-00").is_err());
        assert!(validate_month("2026-13").is_err());
        assert!(
            record(
                AuditAction::AttendanceAdminOverride,
                AuditMetadata::AttendanceOverride {
                    changed_count: MAX_CHANGED_COUNT,
                },
            )
            .is_ok()
        );
        assert!(
            record(
                AuditAction::AttendanceAdminOverride,
                AuditMetadata::AttendanceOverride {
                    changed_count: MAX_CHANGED_COUNT + 1,
                },
            )
            .is_err()
        );
        assert!(
            record(
                AuditAction::EventCreated,
                AuditMetadata::EventCreated {
                    creation_mode: EventCreationMode::New,
                    source_event_id: Some("event_1".into()),
                },
            )
            .is_err()
        );
        for creation_mode in [
            EventCreationMode::CancelledRecreate,
            EventCreationMode::EventCopy,
        ] {
            assert!(
                record(
                    AuditAction::EventCreated,
                    AuditMetadata::EventCreated {
                        creation_mode,
                        source_event_id: Some("event_1".into()),
                    },
                )
                .is_ok()
            );
        }
        assert!(
            record(
                AuditAction::EventEdited,
                AuditMetadata::EventEdited {
                    edit_scope: EventEditScope::SingleDaySchedule,
                },
            )
            .is_ok()
        );
    }

    #[test]
    fn sanitizer_removes_forbidden_keys_recursively_and_case_insensitively() {
        let value = json!({
            "Password": "x",
            "safe": {
                "api_token": "x",
                "nested": [{
                    "SESSION_id": "x",
                    "memo": "x",
                    "relink_code_id": "relink_1",
                    "safe_count": 2
                }]
            }
        });
        let sanitized = sanitize_and_serialize(value).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&sanitized).unwrap(),
            json!({
                "safe": { "nested": [{
                    "relink_code_id": "relink_1",
                    "safe_count": 2
                }]}
            })
        );
        for key in ["code", "hmac", "session", "memo", "body", "note"] {
            let sanitized = sanitize_and_serialize(json!({ "nested": [{ key: "x" }] })).unwrap();
            assert_eq!(sanitized, r#"{"nested":[{}]}"#);
        }
        let patterns = sanitize_and_serialize(json!({
            "prefix_code_suffix": "x",
            "safe_hmac_value": "x",
            "my_session_reference": "x",
            "memoized": "x",
            "authorization_header": "x",
            "password_hash": "x",
            "safe": true
        }))
        .unwrap();
        assert_eq!(patterns, r#"{"safe":true}"#);
    }

    #[test]
    fn action_context_requires_only_reviewed_identifier_shapes() {
        assert!(
            AuditRecord::build(
                "aud_0123456789abcdef012345".into(),
                "2026-07-15T00:00:00Z".into(),
                "request_1",
                None,
                None,
                None,
                AuditAction::SessionLogout,
                AuditMetadata::None,
            )
            .is_ok()
        );
        assert!(matches!(
            AuditRecord::build(
                "aud_0123456789abcdef012345".into(),
                "2026-07-15T00:00:00Z".into(),
                "request_1",
                None,
                None,
                Some("session_1"),
                AuditAction::SessionLogout,
                AuditMetadata::None,
            ),
            Err(AuditBuildError::InvalidIdentifier)
        ));
        assert!(
            AuditRecord::build(
                "aud_0123456789abcdef012345".into(),
                "2026-07-15T00:00:00Z".into(),
                "request_1",
                Some("community_1"),
                Some("membership_1"),
                None,
                AuditAction::CalendarFeedTokenGenerated,
                AuditMetadata::None,
            )
            .is_ok()
        );
        assert!(matches!(
            AuditRecord::build(
                "aud_0123456789abcdef012345".into(),
                "2026-07-15T00:00:00Z".into(),
                "request_1",
                None,
                Some("membership_1"),
                Some("event_1"),
                AuditAction::EventCancelled,
                AuditMetadata::None,
            ),
            Err(AuditBuildError::InvalidIdentifier)
        ));
    }

    #[test]
    fn sanitizer_rejects_root_depth_node_and_byte_limits() {
        assert!(matches!(
            sanitize_and_serialize(json!([])),
            Err(AuditBuildError::MetadataRoot)
        ));
        let depth_8 = json!({"a":{"b":{"c":{"d":{"e":{"f":{"g":1}}}}}}});
        assert!(sanitize_and_serialize(depth_8).is_ok());
        let depth_9 = json!({"a":{"b":{"c":{"d":{"e":{"f":{"g":{"h":1}}}}}}}});
        assert!(matches!(
            sanitize_and_serialize(depth_9),
            Err(AuditBuildError::MetadataDepth)
        ));
        let forbidden_depth_9 = json!({
            "PaSsWoRd": [{"a":{"b":{"c":{"d":{"e":{"f":{"g":1}}}}}}}]
        });
        assert!(matches!(
            sanitize_and_serialize(forbidden_depth_9),
            Err(AuditBuildError::MetadataDepth)
        ));
        let at_limit = Value::Object(
            (0..127)
                .map(|index| (format!("safe_{index}"), json!(index)))
                .collect(),
        );
        assert!(sanitize_and_serialize(at_limit).is_ok());
        let over_limit = Value::Object(
            (0..128)
                .map(|index| (format!("safe_{index}"), json!(index)))
                .collect(),
        );
        assert!(matches!(
            sanitize_and_serialize(over_limit),
            Err(AuditBuildError::MetadataNodes)
        ));
        let forbidden_over_limit = json!({
            "SeSsIoN": [(0..127).map(|index| json!({ "safe": index })).collect::<Vec<_>>()]
        });
        assert!(matches!(
            sanitize_and_serialize(forbidden_over_limit),
            Err(AuditBuildError::MetadataNodes)
        ));
        assert!(matches!(
            sanitize_and_serialize(json!({ "safe": "x".repeat(MAX_METADATA_BYTES) })),
            Err(AuditBuildError::MetadataBytes)
        ));
    }

    #[test]
    fn structured_event_contains_only_allowed_fields() {
        let event = format_event(
            "0123456789abcdef",
            AuditAction::MembershipRemoved,
            AuditOutcome::Success,
        );
        assert_eq!(
            event,
            "event=audit.write request_id=0123456789abcdef action=membership.removed outcome=success"
        );
        for forbidden in ["actor", "community", "target", "metadata", "sql", "bind"] {
            assert!(!event.contains(forbidden));
        }
    }

    #[test]
    fn failure_events_contain_only_bounded_operational_fields() {
        let pre_disclosure = format_failure_event(
            "0123456789abcdef",
            AuditAction::CommunityExportAuthorized,
            AuditFailureEvent::PreDisclosure,
            AuditFailureCategory::Storage,
        );
        assert_eq!(
            pre_disclosure,
            "event=audit.pre_disclosure_failed request_id=0123456789abcdef action=community.export_authorized failure_category=storage route_class=class_b"
        );
        let secondary = format_failure_event(
            "0123456789abcdef",
            AuditAction::SessionLogout,
            AuditFailureEvent::SecondaryWrite,
            AuditFailureCategory::Construction,
        );
        assert_eq!(
            secondary,
            "event=audit.secondary_write_failed request_id=0123456789abcdef action=session.logout failure_category=construction route_class=class_c"
        );
        for event in [pre_disclosure, secondary] {
            for forbidden in [
                "community_id",
                "actor_membership_id",
                "target_id",
                "session_id",
                "metadata",
                "sql",
                "bind",
                "cookie",
            ] {
                assert!(!event.contains(forbidden));
            }
        }
    }
}
