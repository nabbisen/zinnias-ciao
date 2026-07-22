# Quick Start

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [worker-build](https://crates.io/crates/worker-build): `cargo install worker-build`
- [Node.js](https://nodejs.org/) ≥ 18 (for wrangler)
- [Bun](https://bun.sh/) (optional but used in scripts)

## Setup (once)

```sh
bun run setup
# or: npm run setup
```

This installs Wrangler, creates `.dev.vars.dev` with an owner-only random
`HMAC_PEPPER` when the file is absent, applies all D1 migrations to the local
dev database, then seeds one community, one admin user, and a bootstrap invite
code. The secret is never printed. The invite code is printed at the end —
visit `http://localhost:8787/join` to use it.

Running setup again preserves the existing valid pepper, including with
`--reset` or `--yes`, so resetting local data does not silently rotate local
credentials. Setup stops on an invalid, ambiguous, non-regular, or unsafe
secret file; fix that file deliberately instead of replacing it implicitly.

## Development

```sh
bun run dev
```

Opens the worker locally on `http://localhost:8787`.

Local development uses tracked `wrangler.toml` with local D1/KV bindings. Do
not create hosted config files for ordinary local work. `.dev.vars*` and
`.env*` are ignored and must never be committed. Without one valid
`HMAC_PEPPER`, `/healthz` reports not ready and dynamic routes return a fixed
`503`; only the documented immutable/static GET routes remain available.

For hosted Cloudflare staging or production, copy the tracked template to
ignored local config files and put real D1/KV IDs there:

```sh
cp wrangler.toml wrangler.staging.local.toml
cp wrangler.toml wrangler.production.local.toml
git check-ignore -v wrangler.staging.local.toml wrangler.production.local.toml
```

See `docs/src/shared/deployment.md` before running hosted deploy, migration, bootstrap,
or teardown commands.

## Tests

```sh
cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr
```

Domain and contracts tests run as native Rust binaries (no wasm needed).

## Type-check the SSR worker

```sh
cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown
```
