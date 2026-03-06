# GitHub Actions CI/CD Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create two GitHub Actions workflow files that validate PRs (lint + test) and publish a release binary on version tags.

**Architecture:** Two independent workflows in `.github/workflows/`. `ci.yml` runs on every push/PR and gates merges with fmt, clippy, and tests. `release.yml` triggers only on `v*.*.*` tags, builds a release binary, strips it, and publishes it to GitHub Releases.

**Tech Stack:** GitHub Actions, Rust stable toolchain, `actions/cache`, `softprops/action-gh-release@v2`

**Design reference:** `docs/plans/2026-03-06-cicd-design.md`

---

### Task 1: Create the workflow directory

**Files:**
- Create: `.github/workflows/` (directory)

**Step 1: Create the directory**

```bash
mkdir -p .github/workflows
```

**Step 2: Verify**

```bash
ls .github/workflows
```
Expected: empty directory (no output or empty listing)

---

### Task 2: Create `ci.yml`

**Files:**
- Create: `.github/workflows/ci.yml`

**Step 1: Write the workflow file**

Create `.github/workflows/ci.yml` with this exact content:

```yaml
name: CI

on:
  push:
    branches:
      - develop
      - "feature/**"
  pull_request:
    branches:
      - develop
      - main

env:
  CARGO_TERM_COLOR: always

jobs:
  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt

      - name: Check formatting
        run: cargo fmt --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    needs: fmt
    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies
        run: |
          sudo apt-get update -y
          sudo apt-get install -y cmake clang libasound2-dev libudev-dev \
            libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Run clippy
        run: cargo clippy -- -D warnings

  test:
    name: Test
    runs-on: ubuntu-latest
    needs: clippy
    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies
        run: |
          sudo apt-get update -y
          sudo apt-get install -y cmake clang libasound2-dev libudev-dev \
            libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Run tests
        run: cargo test
```

**Step 2: Validate YAML syntax locally (if `actionlint` is available)**

```bash
actionlint .github/workflows/ci.yml
```
Expected: no errors. Skip if `actionlint` is not installed.

Alternatively, validate with Python:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "YAML valid"
```
Expected: `YAML valid`

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add CI workflow for lint and tests"
```

---

### Task 3: Create `release.yml`

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1: Write the workflow file**

Create `.github/workflows/release.yml` with this exact content:

```yaml
name: Release

on:
  push:
    tags:
      - "v*.*.*"

env:
  CARGO_TERM_COLOR: always

permissions:
  contents: write

jobs:
  build-and-release:
    name: Build and publish release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies
        run: |
          sudo apt-get update -y
          sudo apt-get install -y cmake clang libasound2-dev libudev-dev \
            libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-release-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-release-

      - name: Build release binary
        run: cargo build --release

      - name: Strip binary
        run: strip target/release/whisper-type

      - name: Publish GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: target/release/whisper-type
          generate_release_notes: true
```

**Step 2: Validate YAML syntax**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "YAML valid"
```
Expected: `YAML valid`

**Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow for binary publishing on version tags"
```

---

### Task 4: Configure branch protection (manual — GitHub UI)

This step is done in the GitHub repository settings, not in code.

**Steps:**

1. Go to the repository on GitHub → **Settings** → **Branches**
2. Add a rule for `main`:
   - Branch name pattern: `main`
   - Check: **Require a pull request before merging**
   - Check: **Require status checks to pass before merging**
     - Add required checks: `Format`, `Clippy`, `Test`
   - Check: **Do not allow bypassing the above settings**
3. Add a rule for `develop`:
   - Branch name pattern: `develop`
   - Check: **Require status checks to pass before merging**
     - Add required checks: `Format`, `Clippy`, `Test`

**Note:** Status check names (`Format`, `Clippy`, `Test`) must match the `name:` fields in the CI jobs exactly.

---

### Task 5: Verify end-to-end on GitHub

**Step 1: Push the branch and open a PR to `develop`**

```bash
git push origin develop
```

Open a PR on GitHub from a `feature/*` branch (or directly check the Actions tab).

**Step 2: Verify CI runs**

In the GitHub **Actions** tab, confirm:
- `CI` workflow appears and all three jobs (`Format`, `Clippy`, `Test`) pass

**Step 3: Verify release workflow (dry-run)**

Create a test tag locally but **do not push** unless you want a real release:
```bash
git tag -a v0.1.0-test -m "test release"
# To trigger: git push origin v0.1.0-test
# To clean up: git tag -d v0.1.0-test
```

When ready for a real release:
```bash
git tag -a v0.1.0 -m "Initial release: real-time offline speech-to-text for Linux"
git push origin v0.1.0
```

Then check the **Releases** page on GitHub — the binary `whisper-type` should appear as a release asset.
