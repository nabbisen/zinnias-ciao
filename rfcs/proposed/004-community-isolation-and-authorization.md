# RFC 004 — Community Isolation and Authorization

**Status.** Proposed  
**Phase:** M1 / Trust Boundary Foundation  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** AD-1/AD-2 — identity derives from the session's user (idp_subject nullable); offline-write revalidation removed.
**Related roadmap milestone:** M1 / Trust Boundary Foundation  

---

## 1. Summary

This RFC defines the authorization model that prevents one community's data from leaking to another. Community isolation is a core release gate and must be enforced at backend query boundaries, frontend cache boundaries, and error behavior.

---

## 2. Goals

- Enforce active membership for every authenticated community resource.
- Enforce admin/member role per community.
- Return safe generic errors for inaccessible resources.
- Prevent cache key mixing across communities/sessions.
- Provide reusable backend authorization helpers.

---

## 3. Non-Goals

- No complex permission matrix beyond admin/member.
- No public communities.
- No share links to private event details.
- No global admin UI in MVP unless operator-only tooling is introduced separately.

---

## 4. External Behavior

Users only see communities where they are active members. Direct URLs to events in other communities show a generic not-found or no-access screen without confirming the resource exists.

Admins see admin actions only for communities where they are active admins.

---

## 5. Internal Design

Introduce an authorization context built from the current session:

```rust
struct AuthContext {
    user_id: UserId,
    session_id: SessionId,
    memberships: Vec<MembershipSummary>,
}

struct MembershipSummary {
    membership_id: MembershipId,
    community_id: CommunityId,
    role: Role,
}
```

Every handler must derive authorization from `AuthContext`, not from client-submitted role or community claims.

Use helper functions:

```rust
require_member(ctx, community_id) -> MembershipContext
require_admin(ctx, community_id) -> AdminContext
require_event_member(ctx, event_id) -> EventAccessContext
require_event_admin(ctx, event_id) -> EventAdminContext
```

Database queries should join through `community_memberships` and check `removed_at IS NULL`.

---

## 6. Data and API Design

API behavior:

```text
GET /api/communities
GET /api/communities/:community_id/events
GET /api/events/:event_id
```

The server must not trust `community_id` in paths without verifying membership.

Recommended error mapping:

| Case | External response |
|---|---|
| unauthenticated | 401 session expired / join required |
| non-member direct URL | 404 not found or generic no access |
| member invoking admin action | 403 not allowed |
| removed member | 403 no longer has access |

---

## 7. Security, Privacy, and Safety

- Do not log private resource IDs with user-identifying context unless needed for audit/security.
- Do not include resource names in errors for inaccessible resources.
- Frontend cache keys must include user/session and community scope.
- Every state-changing POST re-authorizes from the session before acting (there is no offline write path, AD-1).

---

## 8. Acceptance Criteria

- All event APIs deny non-members.
- Admin APIs deny members.
- Removed members lose access on next API request.
- Direct URL access to inaccessible event does not reveal event title/community.
- Frontend community switch clears or scopes visible data.

---

## 9. Test Plan

- Integration tests for cross-community access denial.
- Property-style tests where random user/community/event IDs are mixed and denied unless membership exists.
- Cache tests for community switching.
- Removed-member test: the next request/POST after removal is denied.

---

## 10. Open Questions / Decisions

Decision: prefer generic 404 for inaccessible event details to reduce information disclosure. Use 403 only where the user already has context, such as visible admin button disabled/denied.
