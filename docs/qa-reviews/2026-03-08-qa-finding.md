#Issues Found

Important
1. language flag always overwrites config (pre-existing but made more visible)
main.rs:74 — config.language = args.language is unconditional because --language has default_value = "de". So a user with "language": "fr" in their config.json gets silently overridden to "de" on every run. The new --whisper-task was correctly added as Option<String> (conditional), making the inconsistency more apparent. Same problem exists for silence_ms at main.rs:75.

2. Silent fallback on typos gives no user feedback
main.rs:83-88 — --whisper-task trnascribe silently runs in transcribe mode with no warning. The plan calls it a "future improvement" but a single warn! call would fix it for free.

Test Gaps
3. No tests for Task serde behavior — config.rs tests don't cover:

Missing whisper_task key → defaults to Transcribe
"translate" → Task::Translate
"Transcribe" (capital T) → serde hard error (because rename_all = "lowercase" is case-sensitive), while the CLI silently defaults. This inconsistency is untested and undocumented.
4. test_serialization_roundtrip (config.rs:185) never sets whisper_task: Task::Translate nor asserts it after roundtrip.

5. Integration test ConfigSnapshot (tests/config_integration.rs) doesn't include whisper_task, so changes to that field's serialization won't be caught there.

Minor
6. Task has no Display impl — The startup log at main.rs:120-127 uses an inline if/else instead of match. If a third variant were added, it would silently print "transcribe".

Recommended Actions (in priority order)
Fix issue #1 — change language and silence_ms to Option<T> with conditional merges (to match how whisper_task and ptt_key are handled)
Add serde tests for Task — especially the case-sensitive deserialization trap
Add warn! for unrecognized --whisper-task values
Update test_serialization_roundtrip to exercise Task::Translate