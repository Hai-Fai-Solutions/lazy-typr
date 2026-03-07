# CI Deduplication Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prevent duplicate CI runs when a push to a feature branch with an open PR triggers both a `push` and a `pull_request` (synchronize) event.

**Architecture:** Add a workflow-level `concurrency` block to `.github/workflows/ci.yml`. Both events share the same concurrency group (keyed on workflow name + git ref); the second event cancels the first. No triggers or jobs are changed.

**Tech Stack:** GitHub Actions YAML

---

### Task 1: Add concurrency block to ci.yml

**Files:**
- Modify: `.github/workflows/ci.yml:13` (after the `env` block, before `jobs`)

**Step 1: Open the file and locate the insertion point**

The file currently looks like:

```yaml
env:
  CARGO_TERM_COLOR: always

jobs:
  fmt:
```

Insert between `env` block and `jobs`.

**Step 2: Add the concurrency block**

Edit `.github/workflows/ci.yml` so it reads:

```yaml
env:
  CARGO_TERM_COLOR: always

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  fmt:
```

**Step 3: Verify the YAML is valid**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo OK
```
Expected: `OK`

**Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: cancel duplicate runs when PR and push fire for same ref"
```

---

### Task 2: Verify behavior in CI

**Step 1: Push the branch and open a PR (or push to an existing PR branch)**

After pushing, observe the Actions tab on GitHub. Two runs will appear briefly; the push-triggered run should be cancelled with status "Cancelled" within seconds of the PR-triggered run starting.

**Step 2: Confirm a lone feature branch push still runs**

Push to a feature branch with no open PR. Confirm CI runs to completion and is not cancelled.

**Step 3: Confirm direct pushes to `develop` are unaffected**

Push to `develop`. Confirm CI runs normally (no `pull_request` event fires for direct pushes, so nothing cancels it).
