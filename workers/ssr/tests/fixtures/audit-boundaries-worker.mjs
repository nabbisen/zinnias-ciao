// Local-only proof of RFC-079 response boundaries. No production data or logs.
function json(value, status = 200, headers = {}) {
  return Response.json(value, {
    status,
    headers: { 'Cache-Control': 'no-store', ...headers },
  });
}

function audit(db, requestId, action) {
  return db.prepare(
    'INSERT INTO proof_boundary_audits (id, request_id, action) VALUES (?1, ?2, ?3)',
  ).bind(crypto.randomUUID(), requestId, action);
}

async function reset(db) {
  await db.batch([
    db.prepare('DELETE FROM proof_boundary_audits'),
    db.prepare("UPDATE proof_boundary_sessions SET revoked=0 WHERE id='session_proof'"),
  ]);
  return json({ ok: true });
}

async function communityExport(db, failAudit) {
  try {
    await audit(
      db,
      failAudit ? 'force_failure' : 'community_success',
      'community.export_authorized',
    ).run();
  } catch {
    return json({ ok: false, error: 'unavailable' }, 503);
  }
  const payload = await db.prepare(
    "SELECT protected_value FROM proof_boundary_payloads WHERE kind='community'",
  ).first();
  return json({ ok: true, payload: payload?.protected_value ?? null });
}

async function matrixAcknowledgement(db, failAudit) {
  try {
    await audit(
      db,
      failAudit ? 'force_failure' : 'matrix_success',
      'calendar_matrix_csv.export_requested',
    ).run();
  } catch {
    return json({ ok: false, error: 'unavailable' }, 503);
  }
  return json({ ok: true });
}

async function logout(db, failAudit) {
  const revoked = await db.prepare(
    "UPDATE proof_boundary_sessions SET revoked=1 WHERE id='session_proof' AND revoked=0",
  ).run();
  if (Number(revoked.meta?.changes ?? 0) !== 1) {
    return json({ ok: false, error: 'revocation_failed' }, 500);
  }
  try {
    await audit(
      db,
      failAudit ? 'force_failure' : 'logout_success',
      'session.logout',
    ).run();
  } catch {
    // Production emits the bounded audit.secondary_write_failed event here.
  }
  return new Response(null, {
    status: 303,
    headers: {
      'Cache-Control': 'no-store',
      Location: '/join',
      'Set-Cookie': 'zinnias_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0',
    },
  });
}

async function state(db) {
  const row = await db.prepare(
    `SELECT
       (SELECT revoked FROM proof_boundary_sessions WHERE id='session_proof') AS revoked,
       (SELECT count(*) FROM proof_boundary_audits WHERE action='community.export_authorized') AS community_audits,
       (SELECT count(*) FROM proof_boundary_audits WHERE action='calendar_matrix_csv.export_requested') AS matrix_audits,
       (SELECT count(*) FROM proof_boundary_audits WHERE action='session.logout') AS logout_audits`,
  ).first();
  return json({
    revoked: Number(row?.revoked ?? -1),
    communityAudits: Number(row?.community_audits ?? -1),
    matrixAudits: Number(row?.matrix_audits ?? -1),
    logoutAudits: Number(row?.logout_audits ?? -1),
  });
}

export default {
  async fetch(request, env) {
    const path = new URL(request.url).pathname;
    if (path === '/health') return json({ ready: true });
    if (path === '/state') return state(env.PROOF_DB);
    if (request.method !== 'POST') return json({ ok: false }, 405);
    if (path === '/reset') return reset(env.PROOF_DB);
    if (path === '/class-b/community/success') return communityExport(env.PROOF_DB, false);
    if (path === '/class-b/community/audit-failure') return communityExport(env.PROOF_DB, true);
    if (path === '/class-b/matrix/success') return matrixAcknowledgement(env.PROOF_DB, false);
    if (path === '/class-b/matrix/audit-failure') return matrixAcknowledgement(env.PROOF_DB, true);
    if (path === '/class-c/logout/success') return logout(env.PROOF_DB, false);
    if (path === '/class-c/logout/audit-failure') return logout(env.PROOF_DB, true);
    return json({ ok: false }, 404);
  },
};
