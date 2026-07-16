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

async function reset(db) {
  await db.batch([
    db.prepare('DELETE FROM audit_log'),
    db.prepare('DELETE FROM proof_multi_writes'),
    db.prepare('DELETE FROM proof_mutations'),
    db.prepare(
      `INSERT INTO proof_mutations (case_name, state, allowed) VALUES
       ('success', 0, 1),
       ('authorization', 0, 0),
       ('audit_failure', 0, 1)`,
    ),
  ]);
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
    return json({ error: 'not found' }, 404);
  },
};
