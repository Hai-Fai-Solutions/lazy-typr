# CI Deduplication Design

**Date:** 2026-03-07
**Status:** Approved

## Problem

The CI workflow triggers on both `push` (to `develop` and `feature/**`) and `pull_request` (targeting `develop` and `main`). When a commit is pushed to a feature branch that has an open PR, GitHub fires both a `push` event and a `pull_request` (synchronize) event. This causes two identical CI runs for the same commit, wasting runner minutes.

## Goal

Run CI exactly once per commit. Feature branch pushes without an open PR must still trigger CI.

## Approach

Add a workflow-level `concurrency` block to `.github/workflows/ci.yml`. GitHub Actions will place both the `push`-triggered run and the `pull_request`-triggered run into the same concurrency group (keyed on workflow name + ref). When the PR synchronize event fires and enters the group, it cancels the already-queued or in-flight push run.

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

## Behavior Matrix

| Event | Branch | PR open? | Result |
|---|---|---|---|
| `push` | `develop` | — | Runs; cancels any in-flight run on same ref |
| `push` | `feature/foo` | No | Runs normally |
| `push` | `feature/foo` | Yes | Starts, then cancelled when PR synchronize event fires |
| `pull_request` | `feature/foo` → `develop` | Yes | Runs; cancels the push run above |
| `pull_request` | `*` → `main` | Yes | Runs normally |

## Trade-offs

- The cancelled push run briefly starts (runner pickup + checkout, ~5–15 s) before being cancelled. Acceptable cost.
- No triggers are removed; no jobs are restructured. The change is additive and minimal.
- `cancel-in-progress: true` also cancels stale runs on the same branch if a second push arrives before CI finishes — a useful side-effect.

## Alternatives Considered

**API-gating job:** A preliminary job calls `gh api` to check for open PRs and skips downstream jobs if one is found. More precise (zero wasted runner time) but adds complexity and an API call on every run. Rejected in favour of simplicity.

**Per-job concurrency:** Same group key applied to each job individually. Equivalent outcome here since all jobs are chained via `needs`. Rejected as unnecessarily verbose.
