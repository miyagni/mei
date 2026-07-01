# Mei

Rust framework for **building AI agents** — the product is the framework, not an app.
UI-agnostic: terminal first, desktop/server too; never browser/WASM.

## Non-negotiable product bars
- **Stupidly low RAM.** A terminal app rendering text has no excuse to exceed ~50 MB —
  and 50 MB is already too much; aim single-digit MB. Stream, don't retain (never buffer
  the whole catalog/transcript/response). Low footprint is the differentiator vs the
  baggage of TS agent frameworks — it's the reason to exist, not asceticism.
- **Minimal output.** Solve it with as little code as the problem takes; LOC is not a score.
- **DX is the product.** It's a framework: the public API's ergonomics decide adoption.
  Judge every design by "is this obvious and pleasant for someone building an agent?"

## Workflow (per feature)
structure (skeleton, no bodies) → review → implementation → review → repeat.
The architecture gate comes *before* any concrete code, and DX is a first-class
acceptance criterion of the skeleton. Build incrementally by parts; no aspirational
docs, no upfront task dumps. Claude generates; the owner reviews rigorously.

## Workspace (build order: session → provider → agent last)
- `mei-config`   — shared base; resolves the config dir (`MEI_GLOBAL_CONFIG_DIR`).
- `mei-session`  — transcript API (Linear + Tree sessions).
- `mei-provider` — auth + credential store, model catalog, request adapter
                   (wire × transport → `ModelEvent` stream).
- `mei-agent`    — the agent loop (built last).
- `mei`          — harness/binary; consumes the libs. **Tool execution lives here,
                   never in the libs** (the core only emits tool-call requests).

## Commands
```bash
cargo test -p <crate>
cargo clippy --all-targets      # keep it warning-free
```
- Catalog is feature-gated: `coding` (default) / `image` / `all`. The data is `&'static`,
  **generated** by the internal codegen bin — never hand-edit `catalog/{coding,image,all}.rs`:
  `cargo run -p mei-provider --features codegen --bin mei-codegen`
- The real provider test is `#[ignore]`d and reads env
  `MEI_TEST_BASE_URL` / `MEI_TEST_API_KEY` / `MEI_TEST_MODEL`:
  `cargo test -p mei-provider -- --ignored`

## Gotchas
- **cargo on PowerShell:** compile/progress output goes to **stderr**; piping `2>&1`
  wraps it in a `NativeCommandError` that looks like a failure even on success. Judge
  by the `test result:` / `Finished` line or the exit code, not the red wrapper.

## VCS: jj (colocated), not git
Use `jj`, not `git`. In PowerShell quote `@-` as `'@-'`.
Prefix `jj describe`/`split`/`commit` with `JJ_EDITOR=true` and pass `-m` so jj
doesn't open an interactive editor and hang the shell. Atomic split:
`JJ_EDITOR=true jj split <path> -m "..."` puts those paths in a new commit, the
rest stays in `@`.
Push: `jj bookmark set main -r @ && jj git push --remote origin --bookmark main`.
Remote is **miyagni/mei** (not `hakenshi/mei`). Commits: Conventional Commits, atomic,
one story each.

## Library code style (the lib crates are an SDK)
- No `println!`/`eprintln!`/`dbg!`/`panic!` — return `Result` or data.
- Errors via `thiserror`; JSON via `serde`/`serde_json` (no hand-rolled parsers).
- HTTP is async (`reqwest` + `tokio`): `reqwest` with `default-features = false` +
  `rustls-tls` (never native-tls), only the features a turn needs (`stream`). Lean
  within async — trim what each dep drags in at runtime.
- **Never fake-fill genuinely-absent data.** In Rust the defensive null shows up as
  Option-coalescing: `.unwrap_or_default()`, `.unwrap_or(vec![]/0/"")`, a `None => <default>`
  arm, or `.ok()` swallowing an error — all papering over a value that didn't arrive. Keep
  absence honest: `Option`/`None`, or return an error; let it break loud. `Option` only when
  genuinely nullable. (A default IS fine when absence is legitimately empty — e.g.
  `#[serde(default)]` on a field that's truly optional.)
- All code/comments/strings in **English** (chat stays pt-BR).
- Tests validate real behavior and leave nothing on disk — use `tempfile::tempdir()`,
  never the project/home dir.
