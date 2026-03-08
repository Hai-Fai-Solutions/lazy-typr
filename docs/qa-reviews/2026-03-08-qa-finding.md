# QA Findings

## Issue Entities

1. `ISSUE-1A`  
Title: `language` flag always overwrites config  
Status: `Solved`  
Details: `language` no longer overwrites config unless `--language` is explicitly provided.

2. `ISSUE-1B`  
Title: `silence_ms` flag always overwrites config  
Status: `Open`  
Details: `silence_ms` is still unconditionally merged from CLI default.

3. `ISSUE-2`  
Title: Silent fallback on `--whisper-task` typo  
Status: `Open`  
Details: `--whisper-task trnascribe` silently falls back without warning.

4. `ISSUE-3`  
Title: Missing serde tests for `Task` behavior  
Status: `Open`  
Details:
- missing `whisper_task` key defaults to `Transcribe`
- `"translate"` maps to `Task::Translate`
- `"Transcribe"` (capital T) serde failure behavior is asserted and documented

5. `ISSUE-4`  
Title: Roundtrip test does not exercise `Task::Translate`  
Status: `Open`  
Details: `test_serialization_roundtrip` does not include/assert the non-default task variant.

6. `ISSUE-5`  
Title: Integration snapshot does not include `whisper_task`  
Status: `Open`  
Details: `ConfigSnapshot` in integration tests misses this field, so serialization regressions may go unnoticed.

7. `ISSUE-6`  
Title: `Task` has no `Display` impl  
Status: `Open`  
Details: startup log formatting uses inline logic instead of a type-level display mapping.

## Recommended Actions (Updated Priority)

1. Fix `ISSUE-1B`: make `silence_ms` conditional like `language`.
2. Fix `ISSUE-2`: add warning/error handling for unrecognized `--whisper-task` values.
3. Fix `ISSUE-3`: add serde tests for `Task` (including case-sensitivity behavior).
4. Fix `ISSUE-4` and `ISSUE-5`: extend roundtrip and integration snapshot tests to cover `whisper_task`.
5. Fix `ISSUE-6`: add `Display` for `Task` and use it in startup logging.
