# Design: Language Override Fix from 2026-03-08 QA Findings

**Date:** 2026-03-08  
**Status:** Proposed

## Source

- QA report: `docs/qa-reviews/2026-03-08-qa-finding.md`

## Scope Decision

Only the `language` override fix was selected for this design from the QA report. All other findings are intentionally out of scope for this document.

## Goals

1. Preserve config file `language` unless `--language` is explicitly passed.
2. Add regression coverage for language merge precedence.

## Non-Goals

- No behavior changes to audio/transcription pipeline.
- No config migration format changes.
- No scope for `silence_ms` or `whisper_task` in this design.

## Finding to Fix Now

### F1: CLI default overwrites persisted language

Current behavior in `src/main.rs`:
- `--language` has a clap default (`"de"`) and is always merged into config.

This prevents a user’s saved `language` value from surviving a normal run with no flags.

### Design

- Change CLI args in `Args`:
  - `language: Option<String>` (remove clap `default_value`)
- Update merge logic to conditional assignment:
  - `if let Some(language) = args.language { config.language = language; }`
- Keep `Config::default()` value unchanged (`language = "de"`).

### Expected UX After Change

- No config file + no CLI flags: app still uses default language via `Config::default()`.
- Config file exists + no CLI flags: app uses persisted language.
- `--language` provided: CLI overrides language for current run.

## Test Design (Now)

### Unit/Integration coverage additions

1. `tests/cli_config_merge.rs` (new)
- Add tests for merge precedence:
  - `no_cli_flags_keeps_config_values`
  - `cli_language_overrides_config`
  - `cli_without_language_does_not_mutate_language`

2. `src/main.rs` refactor for testability
- Extract merge into a pure helper (example: `fn apply_cli_overrides(config: &mut Config, args: &Args)`), then unit-test helper semantics.
- Keep parsing/runtime behavior unchanged.

3. `README.md` update
- Revise CLI help text to avoid implying `--language` hard default at clap-parse level.
- Clarify: defaults come from config/default config.

## Rollout Steps

1. Implement `--language` argument type + conditional merge change.
2. Add focused merge tests for language precedence.
3. Update README usage/config wording for language default source.
4. Run `cargo test` and verify no regressions.
5. Land changes in one PR with QA report reference.

## Risks and Mitigations

- Risk: Help output change may confuse existing users.
  - Mitigation: Document precedence clearly in README and release notes.
- Risk: Language merge behavior can regress in later CLI refactors.
  - Mitigation: Keep language precedence tests as regression guard.

## Acceptance Criteria

1. With config `language=fr`, running `whisper-type` without flags keeps `fr` at runtime.
2. `whisper-type --language en` overrides language.
3. Automated tests fail if unconditional language overwrite behavior returns.

## Questions Requiring Your Decision

1. For README behavior wording, do you want “default language is de” to remain user-facing, or should we phrase it as “default comes from config file (or built-in defaults if config is absent)”?
