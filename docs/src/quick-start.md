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

This installs wrangler and applies the D1 migration to the local dev database.

## Development

```sh
bun run dev
```

Opens the worker locally on `http://localhost:8787`.

## Tests

```sh
cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts
```

Domain and contracts tests run as native Rust binaries (no wasm needed).

## Type-check the SSR worker

```sh
cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown
```
