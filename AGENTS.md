# Working on Slotshot

- Read README.md, docs/PRODUCT.md, and docs/ARCHITECTURE.md before changing behavior.
- Implement only the requested task. Task 001 is foundation only; capture, slots,
  shortcuts, and clipboard features remain planned.
- Keep one small React/TypeScript frontend, plain CSS, and a Rust/Tauri backend.
  Prefer direct functions and local state; add abstractions only for real needs.
- Keep runtime dependencies minimal. Justify new crates, npm packages, plugins,
  and permissions. No server, accounts, database, analytics, or cloud services.
- Support macOS and Windows deliberately. Use Rust path APIs, isolate native code
  behind platform gates when needed, and document platform-specific limitations.
- Treat screenshots as private local data. Keep OS operations in Rust, validate
  frontend input, and grant only the Tauri permissions a feature actually uses.
- Use npm and preserve package-lock.json. Commit Cargo.lock once Rust can generate
  it; do not ignore it or invent a lockfile.
- Run `npm ci`, `npm run check`, and `npm run build`. With Rust installed, also run
  `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`,
  `cargo test --manifest-path src-tauri/Cargo.toml`, and
  `npm run tauri -- build --debug --no-bundle`.
- Add focused tests with new behavior, particularly selection rules, persistence,
  and OS error handling. Do not add placeholder tests just to claim coverage.
- Smoke-test relevant UI and native workflows on both platforms when possible.
  Report missing prerequisites and unverified behavior honestly.
- Update docs when scope or setup changes. Use conventional commits, preserve
  existing work and remotes, and never commit generated output or private data.
