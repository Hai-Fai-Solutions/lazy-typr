# Branch Flow Simplification — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove `develop` from all CI triggers and project docs; `feature/*` and `bugfix/*` merge directly to `main`.

**Architecture:** Two targeted file edits — `ci.yml` (trigger block) and `CLAUDE.md` (branching table). No logic changes, no new files.

**Tech Stack:** GitHub Actions YAML, Markdown

**Design doc:** `docs/plans/2026-03-07-branch-flow-design.md`

---

### Task 1: Fix CI push and PR triggers

**Files:**
- Modify: `.github/workflows/ci.yml:3-11`

**Step 1: Open the file and confirm current triggers**

Read `.github/workflows/ci.yml` lines 1-18.
Expected: `push.branches` contains `develop` and `feature/**`; `pull_request.branches` contains `develop` and `main`.

**Step 2: Edit `push.branches`**

Replace:
```yaml
  push:
    branches:
      - develop
      - "feature/**"
```
With:
```yaml
  push:
    branches:
      - "feature/**"
      - "bugfix/**"
```

**Step 3: Edit `pull_request.branches`**

Replace:
```yaml
  pull_request:
    branches:
      - develop
      - main
```
With:
```yaml
  pull_request:
    branches:
      - main
```

**Step 4: Verify the full `on:` block looks correct**

Read `.github/workflows/ci.yml` lines 1-20.
Expected:
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

**Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: remove develop branch, add bugfix/** trigger"
```

---

### Task 2: Update CLAUDE.md branching section

**Files:**
- Modify: `CLAUDE.md` (Branching section)

**Step 1: Find the branching table in CLAUDE.md**

Search for `develop` in `CLAUDE.md`. Expected: found in the Branching section under `## Developer lifecycle`.

**Step 2: Replace the branching table**

Replace:
```markdown
- `main` — stable, tagged releases
- `develop` — integration branch; all PRs target here
- `feature/*` — short-lived branches off `develop`
```
With:
```markdown
- `main` — stable, tagged releases; all PRs target here
- `feature/*` or `bugfix/*` — short-lived branches off `main`
```

**Step 3: Update the Releases line**

Replace:
```markdown
Releases: PR `develop` → `main`, then `git tag -a vX.Y.Z`.
```
With:
```markdown
Releases: PR `feature/*` or `bugfix/*` → `main`; cocogitto auto-bumps version and tag on merge.
```

**Step 4: Verify no remaining references to `develop`**

Run:
```bash
grep -n "develop" CLAUDE.md
```
Expected: no matches.

**Step 5: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update branching model — remove develop, add bugfix/*"
```
