# Working on Slotshot

- Read README.md, docs/PRODUCT.md, docs/ARCHITECTURE.md, and docs/DESIGN.md before changing behavior. Implement only the requested task.
- Use the primary local repository at ~/Developer/Screenshot. Do not create or
  use additional Git worktrees or isolated project checkouts unless explicitly
  requested. Develop sequentially through regular feature branches in the primary
  checkout. If the current execution environment is a separate worktree, stop and
  request a local-workspace session instead.
- Develop sequentially with small, meaningful conventional commits.
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
- For application/configuration/dependency changes, run `npm ci`, `npm run check`,
  and `npm run build`. With Rust installed, also run
  `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings`,
  `cargo test --manifest-path src-tauri/Cargo.toml --locked`, and
  `npm run tauri -- build --debug --no-bundle -- --locked`.
- For documentation-only changes, check changed Markdown formatting, local links,
  specification consistency, and `git diff --check`. Dependency installation,
  application builds, and native smoke tests are not required for that scope.
- Add focused tests with new behavior, particularly selection rules, persistence,
  and OS error handling. Do not add placeholder tests just to claim coverage.
- Smoke-test relevant UI and native workflows on both platforms when possible.
  Report missing prerequisites and unverified behavior honestly.
- Update docs when scope or setup changes. Use conventional commits, preserve
  existing work and remotes, and never commit generated output or private data.

## Binding file-based architecture

- Final capture destinations: Store in Slot and Save to Device. No system
  clipboard Copy/Paste feature, multi-image clipboard transfer, paste simulation,
  clipboard monitoring/restoration, or interception of standard Cmd/Ctrl+V.
- Use 1–10 numbered slots, default 5, each with one full-resolution PNG persisted
  in the OS-standard application data location across restarts. Rust owns paths,
  file operations, safe/atomic replacement, and cleanup; no database.
- Replace/Clear permanently removes old PNGs and obsolete thumbnails. Increasing
  count creates empty slots. Decreasing count permanently removes every image
  and thumbnail above the new count; confirm first when affected slots are
  occupied. Never silently retain screenshots in hidden/removed slots.
- Exports are independent PNG files in the user's configured directory, chosen
  during initial setup with a sensible default and changeable in Settings. Save
  Selected exports multiple slots as separate PNGs. Slot deletion/replacement
  must never delete separately exported screenshots.
- Native drag-out offers selected PNGs together in one file drag and preserves
  source files. CrabNebula drag-rs is used directly from Rust with
  copy semantics and an operation guard; receiving-app compatibility still needs
  native testing. Test on macOS and Windows, including ChatGPT and
  common file-accepting apps; never promise universal multi-file drop support.
- Preserve approved Template 1: compact dark translucent picker, thumbnails,
  slot numbers, blue selection, full resizing, Save/Clear toolbar, and native
  drag-out as the primary transfer action. No thumbnail context menus. Shift
  selects ranges; Cmd (macOS) / Ctrl (Windows) selects individuals. The reference
  image's obsolete Copy/Paste interactions are superseded by docs/DESIGN.md.

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
