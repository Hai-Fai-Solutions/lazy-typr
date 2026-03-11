# QA Findings

## Issue Entities

1. `ISSUE-1A`
Title: `language` flag always overwrites config
Status: `Solved`
Details: `language` no longer overwrites config unless `--language` is explicitly provided.

2. `ISSUE-1B`
Title: `silence_ms` flag always overwrites config
Status: `Solved`
Details: `silence_ms` is still unconditionally merged from CLI default.

3. `ISSUE-2`
Title: Silent fallback on `--whisper-task` typo
Status: `Solved`
Details: `--whisper-task` removed entirely. Replaced with `--translate` boolean flag — no string to mistype.

4. `ISSUE-3`
Title: Missing serde tests for `Task` behavior
Status: `Obsolete`
Details: `Task` enum was removed. Replaced by `translate: bool`. Serde tests for the new field are covered in `config::tests`.

5. `ISSUE-4`
Title: Roundtrip test does not exercise `Task::Translate`
Status: `Obsolete`
Details: `Task` enum was removed. `translate: true` roundtrip is covered by `test_translate_roundtrips_true`.

6. `ISSUE-5`
Title: Integration snapshot does not include `whisper_task`
Status: `Solved`
Details: `ConfigSnapshot` updated to use `translate: bool` instead of the removed `whisper_task` field.

7. `ISSUE-6`
Title: `Task` has no `Display` impl
Status: `Obsolete`
Details: `Task` enum was removed. Startup log uses `info!("Translate: enabled")` gated on `config.translate`.

## Recommended Actions (Updated Priority)

All issues resolved or obsolete. No open actions remain.
