# Overview

**ciao.zinnias** is a mobile-first, invite-only community schedule-sharing service.
Members check upcoming events, mark attendance, and leave short notes.
Admins manage events, generate invite codes, and maintain the member list.

## Key properties

- Invite-only — no public registration, no passwords in MVP.
- Server-side rendered — works without JavaScript; progressive enhancement only.
- Community-isolated — no data crosses community boundaries.
- Read-only offline — previously viewed pages open without a network.
- Plain-language UI — designed for non-technical community members.

## Stack

| Layer | Technology |
|-------|-----------|
| Runtime | Cloudflare Workers (V8 isolate) |
| Language | Rust (Rust 2024 edition) |
| Database | Cloudflare D1 (SQLite-compatible) |
| Frontend | Plain Rust SSR + minimal plain JS (no browser WASM, no Leptos — AD-1) |
| Auth | Invite-code + HTTP-only cookie session |
