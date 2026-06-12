// ciao.zinnias — progressive enhancement + service worker registration
// AD-1: the app works fully without this script.

'use strict';

// ── Service Worker ────────────────────────────────────────────────────────
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(() => {});
}

// ── Offline banner ────────────────────────────────────────────────────────
// Shown by the SW via the offline fallback page; also toggled here for
// network change events so the banner clears when connectivity returns.
function updateOfflineBanner() {
  const banner = document.getElementById('offline-banner');
  if (!banner) return;
  banner.hidden = navigator.onLine;
}
window.addEventListener('online',  updateOfflineBanner);
window.addEventListener('offline', updateOfflineBanner);
updateOfflineBanner();

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
