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
- Use npm and preserve package-lock.json and src-tauri/Cargo.lock. Use `--locked`
  for Rust checks/builds and update lockfiles only for intentional dependency changes.
- Run `npm ci`, `npm run check`, and `npm run build`. With Rust installed, also run
  `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings`,
  `cargo test --manifest-path src-tauri/Cargo.toml --locked`, and
  `npm run tauri -- build --debug --no-bundle -- --locked`.
- Add focused tests with new behavior, particularly selection rules, persistence,
  and OS error handling. Do not add placeholder tests just to claim coverage.
- Smoke-test relevant UI and native workflows on both platforms when possible.
  Report missing prerequisites and unverified behavior honestly.
- Update docs when scope or setup changes. Use conventional commits, preserve
  existing work and remotes, and never commit generated output or private data.

## No automatic code review

- Never invoke `requesting-code-review` or similar code-review skills unless the
  user explicitly requests them.
- Never launch independent review agents, automated review workflows, or review
  integrations unless the user explicitly requests them. Do not add external
  review as a default phase of any task.
- Normal implementation, self-checking, testing, linting, and build validation
  remain expected.

## No waiting for GitHub CI

- Run relevant local checks before committing. Push changes and create or update
  a PR when the task requires it.
- Check the initial GitHub Actions status at most once. Do not poll, repeatedly
  check, or wait for CI to finish.
- If CI is still pending, report that and finish the task. Clearly report any
  already-known failure; never claim pending checks passed.
- Do not merge while required checks are pending or failing.

## Task completion

- Stop when the requested scope is complete. Do not start additional work or
  launch unnecessary background processes.
- Finish with a concise summary of changes, tests, Git status, PR link, and any
  remaining work.
- Keep the workflow lightweight and appropriate for a small solo-developed
  application.
