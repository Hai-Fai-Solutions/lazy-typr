# CI Paths Allowlist Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `paths` allowlist to `.github/workflows/ci.yml` so CI only runs when output-impacting files change.

**Architecture:** Single-file edit — add `paths` blocks to both `push` and `pull_request` triggers in `ci.yml`. No new files, no new jobs, no logic changes. The existing branch filters are preserved unchanged.

**Tech Stack:** GitHub Actions YAML

---

### Task 1: Add paths allowlist to ci.yml triggers

**Files:**
- Modify: `.github/workflows/ci.yml:3-11`

**Step 1: Verify the current trigger block**

Read `.github/workflows/ci.yml` lines 1–11 and confirm the `on:` section looks like:

```yaml
on:
  push:
    branches:
      - "feature/**"
      - "bugfix/**"
  pull_request:
    branches:
      - main
```

**Step 2: Apply the edit**

Replace the `on:` block with:

```yaml
on:
  push:
    branches:
      - "feature/**"
      - "bugfix/**"
    paths:
      - 'src/**'
      - 'tests/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/**'
  pull_request:
    branches:
      - main
    paths:
      - 'src/**'
      - 'tests/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/**'
```

**Step 3: Validate YAML syntax**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo OK
```
Expected: `OK`

(If python3 is unavailable: `cat .github/workflows/ci.yml` and eyeball indentation — YAML requires consistent 2-space indent.)

**Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: trigger only on output-impacting file changes"
```

---

## Acceptance Criteria Verification

After the PR is merged, manually verify:

1. **Docs-only PR** — open a PR that changes only `docs/**` or `README.md`. CI checks should **not** appear (or should be skipped if a required check is configured with `if: always()`).
2. **Source PR** — open a PR that changes any file under `src/**`. CI should trigger and run all three jobs (`fmt` → `clippy` → `test`).
3. **Workflow PR** — open a PR that changes `.github/workflows/ci.yml`. CI should trigger.
