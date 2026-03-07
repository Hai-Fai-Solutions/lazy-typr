# Design: Cocogitto CI Integration

**Date:** 2026-03-07
**Status:** Approved

## Goal

Automate version bumping and changelog generation by introducing cocogitto (`cog`) into the CI pipeline. Merging to `main` triggers `cog bump --auto`, which updates `Cargo.toml`, regenerates `Cargo.lock`, writes `CHANGELOG.md`, commits, and pushes an annotated tag — which in turn fires the existing `release.yml` to build and publish the GitHub Release.

## Flow

```
merge to main
  └─► bump.yml (GitHub Actions)
        └─► cog bump --auto
              ├─► pre_bump_hook: update Cargo.toml version + cargo generate-lockfile
              ├─► update CHANGELOG.md
              ├─► commit: "chore(version): X.Y.Z [skip ci]"
              └─► push --follow-tags
                    └─► tag v*.*.*  triggers release.yml (existing)
```

## Bump level rules (derived from commit types)

| Commit type | Bump |
|-------------|------|
| `BREAKING CHANGE` footer | major |
| `feat` | minor |
| `fix`, `perf`, anything else | patch |

## Files changed

### New: `cog.toml`

```toml
[changelog]
path = "CHANGELOG.md"
template = "remote"
remote = "github.com"
repository = "lazy-typr"
owner = "Hai-Fai-Solutions"
tag_prefix = "v"

bump_commit_message = "chore(version): {{version}} [skip ci]"

pre_bump_hooks = [
  "sed -i 's/^version = \"[0-9]*\\.[0-9]*\\.[0-9]*\"/version = \"{{version}}\"/' Cargo.toml",
  "cargo generate-lockfile",
]
```

The `[skip ci]` in `bump_commit_message` prevents GitHub Actions from re-triggering `bump.yml` when the bot pushes the bump commit to `main`.

### New: `.github/workflows/bump.yml`

- Trigger: `push` to `main`
- Checkout with `fetch-depth: 0` (cog requires full tag history)
- Install Rust + system deps (needed for `cargo generate-lockfile` in pre_bump_hook)
- Download cocogitto pre-built binary (faster than `cargo install`)
- Configure git identity (`github-actions[bot]`)
- Run `cog bump --auto`
- `git push --follow-tags` using `secrets.COG_TOKEN`

Uses `secrets.COG_TOKEN` (repo-scoped PAT) for both checkout and push. The default `GITHUB_TOKEN` cannot push tags that trigger downstream workflows.

### New: `CHANGELOG.md`

Generated and maintained by `cog changelog`. Committed as part of the initial setup (empty/bootstrapped) and updated automatically on every bump.

### Updated: `.claude/skills/release/SKILL.md`

Remove steps that CI now owns:
- Step 3 (ask for version number) — removed
- Step 4 (edit Cargo.toml, regenerate Cargo.lock, commit) — removed
- Step 7 (create and push git tag) — removed

Remaining flow: verify on `develop` → run tests + clippy → open PR `develop` → `main` → wait for merge confirmation. CI handles the rest.

## Prerequisites

- GitHub repo secret `COG_TOKEN`: a fine-grained or classic PAT with `contents: write` scope on this repository.
- `main` branch protection must allow the PAT to push (or use a bypass rule for the bot).

## What stays unchanged

- `ci.yml` — unchanged, still validates PRs targeting `develop` and `main`
- `release.yml` — unchanged, still triggers on `v*.*.*` tags to build and publish
