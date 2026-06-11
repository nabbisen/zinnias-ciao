# Implementation Order Notes

The safest implementation order is:

1. Bootstrap repository and deployment skeleton.
2. Implement D1 schema and migrations.
3. Implement invite redemption and sessions.
4. Implement authorization middleware and cross-community tests.
5. Implement read-only member Home/Event Detail.
6. Implement status update API and UI.
7. Implement note API and UI.
8. Implement idempotency and error model.
9. Implement offline queue and local cache.
10. Implement admin event creation/cancellation.
11. Implement admin invite/member management.
12. Implement PWA service worker.
13. Harden security, logs, accessibility, and QA gates.
14. Complete deployment/operations handoff.
15. Complete design assets and prototype acceptance.

Rules:

- Do not implement UI paths that bypass authorization middleware.
- Do not add rich media, chat, real-time collaboration, recurring events, or analytics in MVP.
- Do not store session secrets in JavaScript-accessible storage.
- Do not hard-delete normal user-visible event history without an explicit operator-level process.
- Do not rely on browser-only validation for security or data integrity.
