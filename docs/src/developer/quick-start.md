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

This installs wrangler and applies all D1 migrations to the local dev database, then seeds one community, one admin user, and a bootstrap invite code. The invite code is printed at the end — visit `http://localhost:8787/join` to use it.

## Development

```sh
bun run dev
```

Opens the worker locally on `http://localhost:8787`.

Local development uses tracked `wrangler.toml` with local D1/KV bindings. Do
not create hosted config files for ordinary local work.

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
