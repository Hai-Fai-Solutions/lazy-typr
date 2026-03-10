# QA Review: Trigger CI Only For Output-Impacting File Changes

## Finding

Current CI can run for changes that do not affect produced behavior or artifacts. The pipeline should run only when changed files can impact output (runtime behavior, build result, packaging, or CI logic).

## Impact

- Longer feedback loop for non-impacting PRs
- Unnecessary GitHub Actions usage
- Extra queue pressure for code-changing PRs

## Recommendation

Update `.github/workflows/ci.yml` triggers to an allowlist (`paths`) of output-impacting files instead of a docs-focused ignore list.

Typical output-impacting paths:

- `src/**`
- `tests/**`
- `Cargo.toml`
- `Cargo.lock`
- `.github/workflows/**`
- build/release config files used by CI (if present)

## Example Trigger Policy (Allowlist)

```yaml
on:
  push:
    branches:
      - main
      - feature/**
      - bugfix/**
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

Note: keep existing branch filters; replace only path filtering with an output-impacting allowlist tailored to this repo.

## Acceptance Criteria

1. CI is not triggered for docs-only changes (`docs/**`, `README.md`, `CLAUDE.md`) unless another allowlisted file is also changed.
2. CI is triggered when source files or CI workflow files are changed.
3. Existing required checks for output-impacting changes remain unchanged.
