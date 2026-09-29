# Contributing

This crate is a companion web service that renders a pinball focused console for development with Frontbox. The Rust side and web app side
have separate build steps (see below).

## Build

```bash
cargo build
cd web
npm install
npm run build
```

## Develop

Run a machine (or the preview example, which streams fake trace data) alongside the Vite dev server:

```bash
cargo run --example preview   # console server on :3000
cd web && npm run dev         # http://localhost:5173
```

Vite hot-reloads the UI and forwards the `/ws` websocket to the server on port 3000.

## TypeScript types

The types in `web/src/types/generated` are generated from Rust with ts-rs. Don't edit them by hand.
They're regenerated automatically by `npm run dev` and `npm run build`, or manually with:

```bash
cargo run --example export_types
```

Regenerate after changing any type sent to the web app (`Hardware`, `TraceEvent`, or anything in
`src/protocol.rs`), and commit the result.
