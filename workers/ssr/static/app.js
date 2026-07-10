// ciao.zinnias — progressive enhancement + service worker registration
// AD-1: the app works fully without this script.

'use strict';

// ── Service Worker ────────────────────────────────────────────────────────
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(() => {});
}

// ── Offline banner + submit-button contract ────────────────────────────────
// When offline, show the banner AND disable submit buttons on status/note
// forms so users see a clear message rather than a confusing network error.
// The app is read-only while offline by design (RFC-055): authenticated HTML
// is not cached, so pages already in the browser tab are readable but no
// writes can succeed. Disabling buttons makes this contract visible.
//
// AD-1: every form still works without this script. Disabling is enhancement only;
// if JS is off the server returns a normal error on POST which is acceptable.
var OFFLINE_SUBMIT_SELECTOR = 'form[action*="/my-status"] button[type="submit"], '
  + 'form[action*="/my-note"] button[type="submit"], '
  + 'form[action*="/attendance"] button[type="submit"]';
var OFFLINE_TITLE = 'オフラインです。保存はできません。';

function setOfflineSubmitState(isOffline) {
  document.querySelectorAll(OFFLINE_SUBMIT_SELECTOR).forEach(function(btn) {
    if (isOffline) {
      btn.disabled = true;
      if (!btn.dataset.offlineTitle) {
        btn.dataset.offlineTitle = btn.title || '';
      }
      btn.title = OFFLINE_TITLE;
    } else {
      btn.disabled = false;
      if (btn.dataset.offlineTitle !== undefined) {
        btn.title = btn.dataset.offlineTitle;
      }
    }
  });
}

function updateOfflineBanner() {
  var banner = document.getElementById('offline-banner');
  if (banner) banner.hidden = navigator.onLine;
  setOfflineSubmitState(!navigator.onLine);
}
window.addEventListener('online',  updateOfflineBanner);
window.addEventListener('offline', updateOfflineBanner);
updateOfflineBanner();

// ── Community switcher ───────────────────────────────────────────────────
// CSP blocks inline event handlers, so select auto-submit lives here.
// The server-rendered submit button remains visible when JS is disabled or stale.
document.querySelectorAll('form[action="/switch"]').forEach(function(form) {
  var select = form.querySelector('select[name="community"]');
  var button = form.querySelector('button[type="submit"]');
  if (!select) return;
  if (button) button.hidden = true;
  select.addEventListener('change', function() {
    form.submit();
  });
});

// ── Note character counter (progressive enhancement) ─────────────────────
// With JS off the textarea still works; the server enforces the limit.
document.querySelectorAll('textarea[name="note"]').forEach(function(ta) {
  const max = 200;
  const form = ta.closest('form');
  const btn  = form && form.querySelector('button[type="submit"]');
  const hint = form && form.querySelector('.note-counter');

  function update() {
    const len = [...ta.value].length; // Unicode-aware
    if (hint) hint.textContent = len + '/' + max;
    if (btn) btn.disabled = len > max;
    ta.style.borderColor = len > max ? '#FF3B30' : '';
  }
  ta.addEventListener('input', update);
  update();
});

// ── Cache purge on logout ─────────────────────────────────────────────────
// The logout form POSTs to the server; before submission tell the SW to
// clear the private page cache so a subsequent user on the same device
// cannot read cached pages from the previous session (RFC-017 §7).
document.querySelectorAll('form[action="/logout"]').forEach(function(form) {
  form.addEventListener('submit', function() {
    if (navigator.serviceWorker.controller) {
      navigator.serviceWorker.controller.postMessage({ type: 'PURGE_PRIVATE' });
    }
  });
});

// ── Monthly attendance matrix CSV export (RFC-068) ───────────────────────
// Admin-only enhancement. The rendered HTML is the data source; the server
// receives only a metadata audit request before the browser creates the file.
function hardenCsvValue(value) {
  var text = String(value == null ? '' : value);
  return /^[\s]*[=+\-@]/.test(text) ? "'" + text : text;
}

function encodeCsvValue(value) {
  return '"' + hardenCsvValue(value).replace(/"/g, '""') + '"';
}

function matrixCsvFromTable(table) {
  var dates = Array.from(table.querySelectorAll('thead th[data-date]'))
    .map(function(cell) { return cell.dataset.date || ''; });
  var lines = [['member_name'].concat(dates).map(encodeCsvValue).join(',')];

  table.querySelectorAll('tbody tr').forEach(function(row) {
    var member = row.querySelector('[data-member-name]');
    var cells = Array.from(row.querySelectorAll('td[data-export-value]'))
      .map(function(cell) { return cell.dataset.exportValue || ''; });
    lines.push([member ? member.dataset.memberName || '' : ''].concat(cells)
      .map(encodeCsvValue)
      .join(','));
  });

  return '\uFEFF' + lines.join('\r\n') + '\r\n';
}

async function requestMatrixCsvAudit(button) {
  var body = new FormData();
  body.set('token', button.dataset.token || '');
  body.set('month', button.dataset.month || '');
  body.set('export_type', button.dataset.exportType || 'calendar_matrix_csv');
  var res = await fetch(button.dataset.auditUrl || '', {
    method: 'POST',
    body: body,
    credentials: 'same-origin',
    headers: { Accept: 'application/json' },
  });
  if (!res.ok) throw new Error('matrix csv audit failed');
}

function downloadMatrixCsv(csv, filename) {
  var blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
  var url = URL.createObjectURL(blob);
  var link = document.createElement('a');
  link.href = url;
  link.download = filename || 'ciao-attendance.csv';
  link.rel = 'noopener';
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(function() { URL.revokeObjectURL(url); }, 0);
}

document.querySelectorAll('[data-calendar-matrix-export-button]').forEach(function(button) {
  button.addEventListener('click', async function() {
    var table = document.querySelector('table[data-calendar-matrix-export]');
    var status = document.querySelector('[data-calendar-matrix-export-status]');
    if (!table) return;
    button.disabled = true;
    if (status) status.textContent = '';
    try {
      var csv = matrixCsvFromTable(table);
      await requestMatrixCsvAudit(button);
      downloadMatrixCsv(csv, button.dataset.filename);
    } catch (_) {
      if (status) status.textContent = status.dataset.errorMessage || '';
    } finally {
      button.disabled = false;
    }
  });
});
