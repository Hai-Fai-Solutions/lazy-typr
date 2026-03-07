# Cocogitto CI Integration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire cocogitto into CI so that every merge to `main` automatically bumps the semver version, updates `CHANGELOG.md`, commits, and pushes an annotated tag that fires the existing release workflow.

**Architecture:** `cog.toml` at the repo root configures bump rules and a `pre_bump_hook` that rewrites `Cargo.toml` and regenerates `Cargo.lock`. A new `bump.yml` GitHub Actions workflow runs on push to `main`, calls `cog bump --auto`, and pushes via a PAT. The bump commit carries `[skip ci]` in its message to prevent re-triggering. The existing `release.yml` (tag-triggered) and `ci.yml` (PR-triggered) are left untouched.

**Tech Stack:** cocogitto (`cog`), GitHub Actions (`cocogitto/cocogitto-action@v3`, `dtolnay/rust-toolchain@stable`), `sed`, `cargo generate-lockfile`, `git push --follow-tags`

---

## Prerequisites (manual — do before running tasks)

- Create a GitHub fine-grained PAT (or classic PAT with `repo` scope) that can push to `main`.
- Add it as repository secret: **Settings → Secrets → Actions → New secret**, name `COG_TOKEN`.
- If `main` has branch protection, ensure the PAT's owner bypasses push restrictions.

---

### Task 1: Create `cog.toml`

**Files:**
- Create: `cog.toml`

**Step 1: Create the file**

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

Save as `cog.toml` at the repo root.

The `template = "remote"` makes the changelog generate GitHub-linked entries.
`bump_commit_message` embeds `[skip ci]` so GitHub Actions skips `bump.yml` when the bot pushes back to `main`.
The two `pre_bump_hooks` run before cog creates the bump commit: first rewrite the version line in `Cargo.toml`, then regenerate `Cargo.lock`.

**Step 2: Verify cog can read the repo**

```bash
cog check
```

Expected: reports on recent commits, no fatal errors. (Install locally first if needed: `cargo install cocogitto`)

**Step 3: Preview what the next bump would produce**

```bash
cog bump --auto --dry-run
```

Expected: prints the calculated next version (e.g. `0.2.0`) without touching any files. If it says "nothing to bump", all commits since the initial tag are either `chore`/`docs` — that is fine; the hook will fire on the first `feat` or `fix`.

**Step 4: Commit**

```bash
git add cog.toml
git commit -m "chore: add cocogitto config"
```

---

### Task 2: Bootstrap `CHANGELOG.md`

**Files:**
- Create: `CHANGELOG.md`

**Step 1: Generate the initial changelog from existing commits**

```bash
cog changelog > CHANGELOG.md
```

Expected: `CHANGELOG.md` is created with entries grouped by version/type derived from the existing conventional commits.

**Step 2: Inspect the output**

```bash
head -40 CHANGELOG.md
```

Expected: a markdown document with a `## Unreleased` or version-headed section listing `feat:` and `fix:` entries.

**Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: bootstrap CHANGELOG"
```

---

### Task 3: Create `bump.yml` workflow

**Files:**
- Create: `.github/workflows/bump.yml`

**Step 1: Create the workflow file**

```yaml
name: Bump version

on:
  push:
    branches:
      - main

jobs:
  bump:
    name: cog bump --auto
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0          # cog needs full tag history to calculate bump
          token: ${{ secrets.COG_TOKEN }}

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cocogitto
        uses: cocogitto/cocogitto-action@v3

      - name: Configure git identity
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"

      - name: Bump version
        id: bump
        run: |
          if cog bump --auto; then
            echo "bumped=true" >> "$GITHUB_OUTPUT"
          else
            echo "No version bump needed (no feat/fix commits since last tag)"
            echo "bumped=false" >> "$GITHUB_OUTPUT"
          fi

      - name: Push bump commit and tag
        if: steps.bump.outputs.bumped == 'true'
        run: git push --follow-tags
```

Key points:
- `fetch-depth: 0` is required — cog walks the full commit graph to find the last tag and determine the bump level.
- `token: ${{ secrets.COG_TOKEN }}` on checkout sets up the git credential helper so `git push` later uses the PAT automatically.
- The `if cog bump --auto; then …` pattern avoids failing the workflow when there are no bumpable commits (e.g. a `docs:` or `chore:` merge).
- `git push --follow-tags` pushes both the bump commit and the new annotated tag. The tag triggers `release.yml`.

**Step 2: Commit**

```bash
git add .github/workflows/bump.yml
git commit -m "ci: add cocogitto bump workflow"
```

---

### Task 4: Update `/release` skill

**Files:**
- Modify: `.claude/skills/release/SKILL.md`

**Step 1: Read the current skill**

Open `.claude/skills/release/SKILL.md` and note the current steps 3–4 (ask version, edit Cargo.toml) and step 7 (tag).

**Step 2: Replace the file content**

```markdown
---
name: release
description: Cut a new whisper-type release - run tests, open PR develop→main; CI handles version bump, tagging, and publishing
disable-model-invocation: true
---

Steps to cut a release:

1. Verify the working tree is clean (`git status`) and on the `develop` branch. Stop and report if not.

2. Run the full test suite and linter:
   ```
   cargo test
   cargo clippy -- -D warnings
   ```
   Stop if either fails — do not proceed with a broken build.

3. Push the `develop` branch and open a PR from `develop` → `main` with:
   - Title: `Release` (CI determines the exact version from conventional commits)
   - Body: summary of unreleased changes (`git log $(git describe --tags --abbrev=0)..HEAD --oneline`)

4. Wait for the user to confirm the PR is merged.

   CI will automatically:
   - Run `cog bump --auto` to determine the new semver version from commit types
   - Update `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`
   - Commit with `chore(version): X.Y.Z [skip ci]` and push an annotated tag
   - Trigger `release.yml` to build, strip, and publish the GitHub Release

5. Report that the release pipeline is running and share the Actions URL for the user to monitor.
```

**Step 3: Commit**

```bash
git add .claude/skills/release/SKILL.md
git commit -m "chore: simplify release skill — CI owns bump and tag"
```

---

### Task 5: Smoke-test end-to-end (manual verification)

This task requires pushing to the remote — confirm with the user before proceeding.

**Step 1: Push the branch**

```bash
git push origin main
```

Expected: `bump.yml` workflow starts in GitHub Actions.

**Step 2: Check the workflow run**

Navigate to the repo's Actions tab → "Bump version" workflow → latest run.

If there are no `feat`/`fix` commits since the last tag, the workflow will print "No version bump needed" and exit successfully without pushing. That is correct.

To force a test bump: make a small `fix:` commit, push to `develop`, open and merge a PR to `main`, then watch `bump.yml`.

**Step 3: Verify the bump commit**

After a successful bump:
- `git log --oneline -3` shows a `chore(version): X.Y.Z [skip ci]` commit
- `Cargo.toml` version line is updated
- `CHANGELOG.md` has a new version section
- An annotated tag `vX.Y.Z` exists
- `release.yml` started automatically

---

## Rollback

If something goes wrong:
- Delete the pushed tag: `git push --delete origin vX.Y.Z`
- Reset `main` to the commit before the bump (requires force-push or a revert commit)
- Fix `cog.toml` or `bump.yml` and re-trigger
