// Local Worker fixture for required-audit transaction semantics.
function json(value, status = 200) {
  return Response.json(value, {
    status,
    headers: { 'Cache-Control': 'no-store' },
  });
}

function auditId() {
  const bytes = new Uint8Array(12);
  crypto.getRandomValues(bytes);
  return `aud_${Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')}`;
}

function conditionalAudit(db, requestId, targetId) {
  return db.prepare(
    `INSERT INTO audit_log
     (id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)
     SELECT ?1, ?2, 'com_proof', 'mem_actor', 'event_template', ?3,
            'event_template.deleted', '{}', '2026-07-16T00:00:00Z'
     WHERE changes() = 1`,
  ).bind(auditId(), requestId, targetId);
}

function unconditionalAudit(db, requestId, targetId) {
  return db.prepare(
    `INSERT INTO audit_log
     (id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)
     VALUES (?1, ?2, 'com_proof', 'mem_actor', 'event_template', ?3,
             'event_template.created', '{}', '2026-07-16T00:00:00Z')`,
  ).bind(auditId(), requestId, targetId);
}

function packageAudit(db, requestId, targetKind, targetId, action) {
  return db.prepare(
    `INSERT INTO audit_log
     (id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)
     SELECT ?1, ?2, 'com_proof', 'mem_actor', ?3, ?4, ?5, '{}', '2026-07-16T00:00:00Z'
     WHERE changes() = 1`,
  ).bind(auditId(), requestId, targetKind, targetId, action);
}

function attendanceAudit(db, requestId) {
  return db.prepare(
    `INSERT INTO audit_log
     (id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)
     SELECT ?1, ?2, 'com_proof', 'mem_actor', 'attendance', 'event_attendance',
            'attendance.admin_override', json_object('changed_count', changes()),
            '2026-07-16T00:00:00Z'
     WHERE changes() BETWEEN 1 AND 10000`,
  ).bind(auditId(), requestId);
}

async function reset(db) {
  await db.batch([
    db.prepare('DELETE FROM audit_log'),
    db.prepare('DELETE FROM proof_multi_writes'),
    db.prepare('DELETE FROM proof_mutations'),
    db.prepare('DELETE FROM proof_event_parts'),
    db.prepare('DELETE FROM proof_event_headers'),
    db.prepare('DELETE FROM proof_attendance'),
    db.prepare('DELETE FROM proof_calendar_tokens'),
    db.prepare('DELETE FROM proof_edit_days'),
    db.prepare('DELETE FROM proof_edit_events'),
    db.prepare('DELETE FROM proof_occurrence_exceptions'),
    db.prepare('DELETE FROM proof_occurrence_days'),
    db.prepare(
      `INSERT INTO proof_mutations (case_name, state, allowed) VALUES
       ('success', 0, 1),
       ('authorization', 0, 0),
       ('audit_failure', 0, 1)`,
    ),
  ]);
}

async function packageState(db, targetId) {
  const row = await db.prepare(
    `SELECT
       (SELECT count(*) FROM proof_event_headers) AS event_headers,
       (SELECT count(*) FROM proof_event_parts) AS event_parts,
       (SELECT count(*) FROM proof_attendance) AS attendance_rows,
       (SELECT count(*) FROM proof_calendar_tokens WHERE active=1) AS active_tokens,
       (SELECT count(*) FROM audit_log WHERE target_id=?1) AS audits`,
  ).bind(targetId).first();
  return {
    eventHeaders: Number(row?.event_headers ?? -1),
    eventParts: Number(row?.event_parts ?? -1),
    attendanceRows: Number(row?.attendance_rows ?? -1),
    activeTokens: Number(row?.active_tokens ?? -1),
    audits: Number(row?.audits ?? -1),
  };
}

async function eventCreateCase(db, failAudit) {
  const requestId = failAudit ? 'proof-audit-failure' : 'proof-event-create';
  const targetId = failAudit ? 'event_failure' : 'event_success';
  const statements = [
    db.prepare('INSERT INTO proof_event_headers (id, allowed) VALUES (?1, 1)').bind(targetId),
    db.prepare(
      'INSERT INTO proof_event_parts (id, event_id) SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM proof_event_headers WHERE id=?2)',
    ).bind(`${targetId}_part_1`, targetId),
    db.prepare(
      'INSERT INTO proof_event_parts (id, event_id) SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM proof_event_headers WHERE id=?2)',
    ).bind(`${targetId}_part_2`, targetId),
    packageAudit(db, requestId, 'event', targetId, 'event.created'),
  ];
  try {
    const results = await db.batch(statements);
    return {
      batchSucceeded: true,
      statementChanges: results.map((result) => Number(result.meta?.changes ?? -1)),
      state: await packageState(db, targetId),
    };
  } catch {
    return { batchSucceeded: false, statementChanges: [], state: await packageState(db, targetId) };
  }
}

async function attendanceCase(db, replay) {
  const payload = JSON.stringify([
    { cell_id: 'cell_1', status: 'going' },
    { cell_id: 'cell_2', status: 'attended' },
  ]);
  const statements = [
    db.prepare(
      `WITH submitted AS (
         SELECT json_extract(value, '$.cell_id') AS cell_id,
                json_extract(value, '$.status') AS status FROM json_each(?1)
       )
       INSERT INTO proof_attendance (cell_id, status)
       SELECT cell_id, status FROM submitted WHERE true
       ON CONFLICT(cell_id) DO UPDATE SET status=excluded.status
       WHERE proof_attendance.status IS NOT excluded.status`,
    ).bind(payload),
    attendanceAudit(db, replay ? 'proof-attendance-replay' : 'proof-attendance'),
  ];
  const results = await db.batch(statements);
  const audit = await db.prepare(
    `SELECT metadata_json FROM audit_log WHERE request_id=?1 ORDER BY created_at DESC LIMIT 1`,
  ).bind(replay ? 'proof-attendance-replay' : 'proof-attendance').first();
  return {
    statementChanges: results.map((result) => Number(result.meta?.changes ?? -1)),
    changedCount: audit ? Number(JSON.parse(audit.metadata_json).changed_count) : 0,
    state: await packageState(db, 'event_attendance'),
  };
}

async function calendarRotateFailure(db) {
  await db.prepare("INSERT INTO proof_calendar_tokens (id, active) VALUES ('old_token', 1)").run();
  try {
    await db.batch([
      db.prepare('UPDATE proof_calendar_tokens SET active=0 WHERE active=1'),
      db.prepare("INSERT INTO proof_calendar_tokens (id, active) VALUES ('new_token', 1)"),
      packageAudit(db, 'proof-audit-failure', 'calendar_feed', null, 'calendar_feed.token_generated'),
    ]);
    return { batchSucceeded: true, state: await packageState(db, null) };
  } catch {
    return { batchSucceeded: false, state: await packageState(db, null) };
  }
}

async function editEligibilityLoss(db) {
  await db.batch([
    db.prepare("INSERT INTO proof_edit_events (id, title) VALUES ('edit_event', 'old title')"),
    db.prepare("INSERT INTO proof_edit_days (id, event_id, day_date, occurrence_status) VALUES ('edit_day_1', 'edit_event', '2026-08-01', 'scheduled')"),
    db.prepare("INSERT INTO proof_edit_days (id, event_id, day_date, occurrence_status) VALUES ('edit_day_2', 'edit_event', '2026-08-02', 'scheduled')"),
  ]);
  const results = await db.batch([
    db.prepare(
      `UPDATE proof_edit_days SET day_date='2026-09-01'
       WHERE event_id='edit_event'
         AND (SELECT count(*) FROM proof_edit_days WHERE event_id='edit_event')=1`,
    ),
    db.prepare(
      `UPDATE proof_edit_events SET title='new title'
       WHERE id='edit_event' AND title IS NOT 'new title'
         AND (SELECT count(*) FROM proof_edit_days WHERE event_id='edit_event')=1
         AND EXISTS (SELECT 1 FROM proof_edit_days
                     WHERE event_id='edit_event' AND day_date='2026-09-01'
                       AND occurrence_status='scheduled')`,
    ),
    packageAudit(db, 'proof-edit-eligibility-loss', 'event', 'edit_event', 'event.edited'),
  ]);
  const state = await db.prepare(
    `SELECT
       (SELECT title FROM proof_edit_events WHERE id='edit_event') AS title,
       (SELECT count(*) FROM proof_edit_days WHERE event_id='edit_event') AS day_count,
       (SELECT count(*) FROM proof_edit_days WHERE event_id='edit_event' AND day_date='2026-09-01') AS requested_days,
       (SELECT count(*) FROM audit_log WHERE request_id='proof-edit-eligibility-loss') AS audits`,
  ).first();
  return {
    statementChanges: results.map((result) => Number(result.meta?.changes ?? -1)),
    state: {
      title: state?.title,
      dayCount: Number(state?.day_count ?? -1),
      requestedDays: Number(state?.requested_days ?? -1),
      audits: Number(state?.audits ?? -1),
    },
  };
}

async function occurrenceCase(db, mode) {
  const failAudit = mode === 'audit-failure';
  const replay = mode === 'replay';
  const dayId = failAudit ? 'occurrence_failure' : 'occurrence_success';
  const requestId = failAudit ? 'proof-audit-failure' : `proof-occurrence-${mode}`;
  if (!replay) {
    await db.prepare(
      'INSERT INTO proof_occurrence_days (id, occurrence_status) VALUES (?1, ?2)',
    ).bind(dayId, 'scheduled').run();
  }
  try {
    const results = await db.batch([
      db.prepare(
        `UPDATE proof_occurrence_days SET occurrence_status='cancelled'
         WHERE id=?1 AND occurrence_status='scheduled'`,
      ).bind(dayId),
      db.prepare(
        `INSERT INTO proof_occurrence_exceptions (day_id)
         SELECT ?1 WHERE changes()=1
         ON CONFLICT(day_id) DO NOTHING`,
      ).bind(dayId),
      packageAudit(db, requestId, 'event_day', dayId, 'event.occurrence_cancelled'),
    ]);
    return {
      batchSucceeded: true,
      statementChanges: results.map((result) => Number(result.meta?.changes ?? -1)),
      state: await occurrenceState(db, dayId),
    };
  } catch {
    return { batchSucceeded: false, statementChanges: [], state: await occurrenceState(db, dayId) };
  }
}

async function occurrenceState(db, dayId) {
  const state = await db.prepare(
    `SELECT
       (SELECT occurrence_status FROM proof_occurrence_days WHERE id=?1) AS status,
       (SELECT count(*) FROM proof_occurrence_exceptions WHERE day_id=?1) AS exceptions,
       (SELECT count(*) FROM audit_log WHERE target_id=?1) AS audits`,
  ).bind(dayId).first();
  return {
    status: state?.status,
    exceptions: Number(state?.exceptions ?? -1),
    audits: Number(state?.audits ?? -1),
  };
}

async function state(db, caseName, targetId) {
  const row = await db.prepare(
    `SELECT
       COALESCE((SELECT state FROM proof_mutations WHERE case_name = ?1), 0) AS business_state,
       (SELECT count(*) FROM proof_multi_writes WHERE case_name = ?1) AS multi_writes,
       (SELECT count(*) FROM audit_log WHERE target_id = ?2) AS audits`,
  ).bind(caseName, targetId).first();
  return {
    businessState: Number(row?.business_state ?? -1),
    multiWrites: Number(row?.multi_writes ?? -1),
    audits: Number(row?.audits ?? -1),
  };
}

async function conditionalCase(db, caseName, requestId, targetId) {
  const statements = [
    db.prepare(
      `UPDATE proof_mutations SET state = 1
       WHERE case_name = ?1 AND state = 0 AND allowed = 1`,
    ).bind(caseName),
    conditionalAudit(db, requestId, targetId),
  ];
  try {
    const results = await db.batch(statements);
    return {
      batchSucceeded: true,
      statementChanges: results.map((result) => Number(result.meta?.changes ?? -1)),
      state: await state(db, caseName, targetId),
    };
  } catch {
    return {
      batchSucceeded: false,
      statementChanges: [],
      state: await state(db, caseName, targetId),
    };
  }
}

async function unconditionalFailure(db) {
  const caseName = 'multi_failure';
  const targetId = 'target_multi_failure';
  try {
    await db.batch([
      db.prepare('INSERT INTO proof_multi_writes (id, case_name) VALUES (?1, ?2)')
        .bind('write_multi_failure', caseName),
      unconditionalAudit(db, 'proof-audit-failure', targetId),
    ]);
    return { batchSucceeded: true, state: await state(db, caseName, targetId) };
  } catch {
    return { batchSucceeded: false, state: await state(db, caseName, targetId) };
  }
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.hostname !== '127.0.0.1' && url.hostname !== 'localhost') {
      return json({ error: 'loopback only' }, 403);
    }
    if (request.method === 'GET' && url.pathname === '/health') {
      return json({ ready: true });
    }
    if (request.method !== 'POST') {
      return json({ error: 'not found' }, 404);
    }
    if (url.pathname === '/reset') {
      await reset(env.PROOF_DB);
      return json({ reset: true });
    }
    if (url.pathname === '/conditional/success' || url.pathname === '/conditional/replay') {
      return json(await conditionalCase(env.PROOF_DB, 'success', 'proof-success', 'target_success'));
    }
    if (url.pathname === '/conditional/authorization') {
      return json(await conditionalCase(
        env.PROOF_DB,
        'authorization',
        'proof-authorization',
        'target_authorization',
      ));
    }
    if (url.pathname === '/conditional/audit-failure') {
      return json(await conditionalCase(
        env.PROOF_DB,
        'audit_failure',
        'proof-audit-failure',
        'target_audit_failure',
      ));
    }
    if (url.pathname === '/unconditional/audit-failure') {
      return json(await unconditionalFailure(env.PROOF_DB));
    }
    if (url.pathname === '/package4/event/success') {
      return json(await eventCreateCase(env.PROOF_DB, false));
    }
    if (url.pathname === '/package4/event/audit-failure') {
      return json(await eventCreateCase(env.PROOF_DB, true));
    }
    if (url.pathname === '/package4/attendance/success') {
      return json(await attendanceCase(env.PROOF_DB, false));
    }
    if (url.pathname === '/package4/attendance/replay') {
      return json(await attendanceCase(env.PROOF_DB, true));
    }
    if (url.pathname === '/package4/calendar/audit-failure') {
      return json(await calendarRotateFailure(env.PROOF_DB));
    }
    if (url.pathname === '/package4/edit/eligibility-loss') {
      return json(await editEligibilityLoss(env.PROOF_DB));
    }
    if (url.pathname === '/package4/occurrence/success') {
      return json(await occurrenceCase(env.PROOF_DB, 'success'));
    }
    if (url.pathname === '/package4/occurrence/replay') {
      return json(await occurrenceCase(env.PROOF_DB, 'replay'));
    }
    if (url.pathname === '/package4/occurrence/audit-failure') {
      return json(await occurrenceCase(env.PROOF_DB, 'audit-failure'));
    }
    return json({ error: 'not found' }, 404);
  },
};
