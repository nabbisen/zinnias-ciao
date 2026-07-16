// Local Worker fixture for the audit change assertion proof.
const OPERATION_ID_PATTERN = /^ast_[A-Za-z0-9_-]{22}$/;

function operationId() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  const encoded = btoa(String.fromCharCode(...bytes))
    .replaceAll('+', '-')
    .replaceAll('/', '_')
    .replace(/=+$/u, '');
  const value = `ast_${encoded}`;
  if (!OPERATION_ID_PATTERN.test(value)) {
    throw new Error('invalid internal assertion identifier');
  }
  return value;
}

function json(value, status = 200) {
  return Response.json(value, {
    status,
    headers: { 'Cache-Control': 'no-store' },
  });
}

async function reset(db) {
  await db.batch([
    db.prepare('DELETE FROM proof_audits'),
    db.prepare('DELETE FROM proof_dependents'),
    db.prepare('DELETE FROM audit_change_assertions'),
    db.prepare('DELETE FROM proof_items'),
    db.prepare('DELETE FROM proof_flow_audits'),
    db.prepare('DELETE FROM proof_flow_sessions'),
    db.prepare('DELETE FROM proof_flow_links'),
    db.prepare('DELETE FROM proof_flow_memberships'),
    db.prepare('DELETE FROM proof_flow_users'),
    db.prepare('DELETE FROM proof_flow_claims'),
    db.prepare(
      `INSERT INTO proof_items (id, case_name, eligible, winner) VALUES
       ('zero-1', 'zero', 0, NULL),
       ('one-1', 'one', 1, NULL),
       ('multi-1', 'multi', 1, NULL),
       ('multi-2', 'multi', 1, NULL),
       ('audit-fail-1', 'audit-fail', 1, NULL),
      ('concurrent-1', 'concurrent', 1, NULL)`,
    ),
    db.prepare(
      `INSERT INTO proof_flow_claims (flow, winner) VALUES
       ('join', NULL), ('relink', NULL), ('join-audit-fail', NULL), ('relink-audit-fail', NULL)`,
    ),
  ]);
}

async function flowSummary(db, flow) {
  const row = await db.prepare(
    `SELECT
       (SELECT count(*) FROM proof_flow_claims WHERE flow=?1 AND winner IS NOT NULL) AS winners,
       (SELECT count(*) FROM proof_flow_users WHERE flow=?1) AS users,
       (SELECT count(*) FROM proof_flow_memberships WHERE flow=?1) AS memberships,
       (SELECT count(*) FROM proof_flow_links WHERE flow=?1) AS links,
       (SELECT count(*) FROM proof_flow_sessions WHERE flow=?1) AS sessions,
       (SELECT count(*) FROM proof_flow_audits WHERE flow=?1) AS audits,
       (SELECT count(*) FROM audit_change_assertions) AS guards`,
  ).bind(flow).first();
  return {
    winners: Number(row?.winners ?? -1),
    users: Number(row?.users ?? -1),
    memberships: Number(row?.memberships ?? -1),
    links: Number(row?.links ?? -1),
    sessions: Number(row?.sessions ?? -1),
    audits: Number(row?.audits ?? -1),
    guards: Number(row?.guards ?? -1),
  };
}

async function runFlow(db, flow) {
  const internalId = operationId();
  const candidate = crypto.randomUUID().replaceAll('-', '');
  const statements = [
    db.prepare('UPDATE proof_flow_claims SET winner=?1 WHERE flow=?2 AND winner IS NULL')
      .bind(candidate, flow),
    db.prepare(
      `INSERT INTO audit_change_assertions (operation_id, changed_count)
       VALUES (?1, changes())`,
    ).bind(internalId),
  ];
  const pushRequired = (statement) => {
    statements.push(
      statement,
      db.prepare(
        `UPDATE audit_change_assertions SET changed_count=changes()
         WHERE operation_id=?1`,
      ).bind(internalId),
    );
  };
  if (flow.startsWith('join')) {
    pushRequired(
      db.prepare('INSERT INTO proof_flow_users (id, flow) VALUES (?1, ?2)')
        .bind(`user-${candidate}`, flow),
    );
    pushRequired(
      db.prepare('INSERT INTO proof_flow_memberships (id, flow, user_id) VALUES (?1, ?2, ?3)')
        .bind(`membership-${candidate}`, flow, `user-${candidate}`),
    );
    pushRequired(
      db.prepare('INSERT INTO proof_flow_links (id, flow, membership_id) VALUES (?1, ?2, ?3)')
        .bind(`link-${candidate}`, flow, `membership-${candidate}`),
    );
    pushRequired(
      db.prepare('INSERT INTO proof_flow_sessions (id, flow, user_id) VALUES (?1, ?2, ?3)')
        .bind(`session-${candidate}`, flow, `user-${candidate}`),
    );
  } else {
    pushRequired(
      db.prepare('INSERT INTO proof_flow_sessions (id, flow, user_id) VALUES (?1, ?2, ?3)')
        .bind(`session-${candidate}`, flow, 'existing-user'),
    );
    statements.push(db.prepare("UPDATE proof_flow_sessions SET user_id=user_id WHERE flow='none'"));
  }
  statements.push(
    db.prepare('INSERT INTO proof_flow_audits (id, flow, outcome) VALUES (?1, ?2, ?3)')
      .bind(`audit-${candidate}`, flow, flow.endsWith('audit-fail') ? 'rejected' : 'ok'),
    db.prepare('DELETE FROM audit_change_assertions WHERE operation_id=?1').bind(internalId),
  );
  try {
    const results = await db.batch(statements);
    return {
      batchSucceeded: true,
      statementCount: statements.length,
      statementChanges: results.map((result) => Number(result.meta?.changes ?? -1)),
    };
  } catch {
    return { batchSucceeded: false, statementCount: statements.length, statementChanges: [] };
  }
}

async function summary(db, caseName) {
  const row = await db
    .prepare(
      `SELECT
         (SELECT count(*) FROM proof_items WHERE case_name = ?1 AND winner IS NOT NULL) AS winners,
         (SELECT count(*) FROM proof_dependents WHERE case_name = ?1) AS dependents,
         (SELECT count(*) FROM proof_audits WHERE case_name = ?1) AS audits,
         (SELECT count(*) FROM audit_change_assertions) AS guards`,
    )
    .bind(caseName)
    .first();
  return {
    winners: Number(row?.winners ?? -1),
    dependents: Number(row?.dependents ?? -1),
    audits: Number(row?.audits ?? -1),
    guards: Number(row?.guards ?? -1),
  };
}

async function runGuardedBatch(db, caseName, winner, auditOutcome = 'ok') {
  const internalId = operationId();
  const statements = [
    db.prepare(
      `UPDATE proof_items
       SET winner = ?1
       WHERE case_name = ?2 AND eligible = 1 AND winner IS NULL`,
    ).bind(winner, caseName),
    db.prepare(
      `INSERT INTO audit_change_assertions (operation_id, changed_count)
       VALUES (?1, changes())`,
    ).bind(internalId),
    db.prepare(
      'INSERT INTO proof_dependents (id, case_name) VALUES (?1, ?2)',
    ).bind(`dep-${caseName}`, caseName),
    db.prepare(
      'INSERT INTO proof_audits (id, case_name, outcome) VALUES (?1, ?2, ?3)',
    ).bind(`aud-${caseName}`, caseName, auditOutcome),
    db.prepare('DELETE FROM audit_change_assertions WHERE operation_id = ?1').bind(internalId),
  ];

  try {
    const results = await db.batch(statements);
    return {
      batchSucceeded: true,
      statementCount: results.length,
      statementChanges: results.map((result) => Number(result.meta?.changes ?? -1)),
    };
  } catch {
    return {
      batchSucceeded: false,
      statementCount: statements.length,
      statementChanges: [],
    };
  }
}

async function runCase(db, caseName) {
  const auditOutcome = caseName === 'audit-fail' ? 'rejected' : 'ok';
  const outcome = await runGuardedBatch(db, caseName, `winner-${caseName}`, auditOutcome);
  return { ...outcome, state: await summary(db, caseName) };
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
    if (url.pathname === '/id-properties') {
      const values = Array.from({ length: 128 }, operationId);
      return json({
        generated: values.length,
        unique: new Set(values).size === values.length,
        valid: values.every((value) => OPERATION_ID_PATTERN.test(value)),
      });
    }
    if (url.pathname === '/case/concurrent') {
      const outcome = await runGuardedBatch(env.PROOF_DB, 'concurrent', 'winner-concurrent');
      return json({ ...outcome, state: await summary(env.PROOF_DB, 'concurrent') });
    }
    if (url.pathname.startsWith('/case/')) {
      const caseName = url.pathname.slice('/case/'.length);
      if (!['zero', 'one', 'multi', 'audit-fail'].includes(caseName)) {
        return json({ error: 'unknown case' }, 404);
      }
      return json(await runCase(env.PROOF_DB, caseName));
    }
    if (url.pathname === '/summary/concurrent') {
      return json(await summary(env.PROOF_DB, 'concurrent'));
    }
    if (url.pathname === '/flow/join/concurrent') {
      return json(await runFlow(env.PROOF_DB, 'join'));
    }
    if (url.pathname === '/flow/relink/concurrent') {
      return json(await runFlow(env.PROOF_DB, 'relink'));
    }
    if (url.pathname === '/flow/join/summary') {
      return json(await flowSummary(env.PROOF_DB, 'join'));
    }
    if (url.pathname === '/flow/relink/summary') {
      return json(await flowSummary(env.PROOF_DB, 'relink'));
    }
    if (url.pathname === '/flow/join/audit-failure') {
      const flow = 'join-audit-fail';
      return json({ ...(await runFlow(env.PROOF_DB, flow)), state: await flowSummary(env.PROOF_DB, flow) });
    }
    if (url.pathname === '/flow/relink/audit-failure') {
      const flow = 'relink-audit-fail';
      return json({ ...(await runFlow(env.PROOF_DB, flow)), state: await flowSummary(env.PROOF_DB, flow) });
    }
    return json({ error: 'not found' }, 404);
  },
};
