# CI/CD Design — whisper-type

Date: 2026-03-06

## Approach

Two GitHub Actions workflow files: one for CI (PR validation), one for release (binary publishing).

## Workflow 1: `ci.yml`

**Triggers:**
- Push to `develop` or `feature/**`
- Pull request targeting `develop` or `main`

**Runner:** `ubuntu-latest`

**System packages (apt):**
- `cmake` — required by whisper-rs to compile whisper.cpp
- `clang` — C++ compiler for whisper.cpp
- `libasound2-dev` — ALSA headers for cpal
- `libxdo-dev` — xdotool linkage

**Cache:** `actions/cache` on `~/.cargo/registry`, `~/.cargo/git`, `target/`; keyed on `Cargo.lock` hash.

**Jobs (sequential):**
1. `cargo fmt --check` — fails PR if formatting is wrong
2. `cargo clippy -- -D warnings` — fails PR on any lint warning
3. `cargo test` — runs all unit and integration tests (no audio hardware or Whisper model required)

## Workflow 2: `release.yml`

**Trigger:** Push of a tag matching `v*.*.*`

**Runner:** `ubuntu-latest`

**Steps:**
1. Checkout repo
2. Install system packages (same as ci.yml)
3. Restore cargo cache
4. `cargo build --release`
5. `strip target/release/whisper-type`
6. Create GitHub Release via `softprops/action-gh-release` using tag annotation as release notes
7. Upload `target/release/whisper-type` as release asset

**Permissions:** `contents: write` (uses auto-provided `GITHUB_TOKEN`, no manual secrets needed)

## Branch Protection (GitHub Settings)

- `main`: require PR, require CI checks to pass, no direct push
- `develop`: require CI checks to pass on PRs

## Decisions

| Decision | Choice | Reason |
|----------|--------|--------|
| Workflow count | 2 | Clear separation: validate vs. publish |
| Release artifact | Static binary | Simplest; no packaging needed yet |
| Slow build mitigation | `actions/cache` on Cargo.lock hash | whisper-rs only recompiles when deps change |
| Secrets | None beyond GITHUB_TOKEN | Fully offline build, no registry publish |
| Self-hosted runners | No | Tests don't need audio hardware or Whisper model |
| Fmt/Clippy | Block PR merge | Enforces code quality from the start |
