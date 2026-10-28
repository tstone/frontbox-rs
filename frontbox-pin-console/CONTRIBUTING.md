# Contributing

This crate is a companion web service that renders a pinball focused console for development with Frontbox.

## Build

```bash
cargo build
```

`build.rs` builds the web app into `web/dist` whenever its sources change, installing its packages first if needed.
It needs Node.js 20.19+ or 22.12+, but not on the PATH: it also looks in nvm, nvm-windows and the default install
location, so a build from PowerShell or the editor works when nvm is only set up in Git Bash. Without a suitable Node
the build warns and embeds the existing `web/dist`. Set `FRONTBOX_SKIP_WEB_BUILD` to skip the web build.

## Develop

Run a machine (or the preview example, a virtual machine that plays a scripted game on a loop) alongside the Vite dev server:

```bash
cargo run --example preview   # console server on :3000
cd web && npm run dev         # http://localhost:5173
```

Vite hot-reloads the UI and forwards the `/ws` websocket to the server on port 3000.

## TypeScript types

The types in `web/src/types/generated` are generated from Rust with ts-rs. Don't edit them by hand.
They're regenerated automatically by `npm run dev` and `npm run build`, but not by `cargo build` (it can't run cargo
from inside a build), so after changing a Rust type run:

```bash
cargo run --example export_types
```

Regenerate after changing any type sent to the web app (`Hardware`, `TraceEvent`, or anything in
`src/protocol.rs`), and commit the result.
