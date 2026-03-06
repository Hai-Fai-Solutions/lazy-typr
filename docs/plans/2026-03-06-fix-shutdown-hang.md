# Plan: Fix application hang on shutdown

## Context

The application hangs after pressing Ctrl+C and never exits. The user reported it does not shut down when started.

## Root Cause (confirmed)

**Primary bug — `text_tx` is not dropped before `typer_handle.join()`**

In [src/main.rs:149](src/main.rs#L149), `text_tx` (the original sender of the text channel) is created and remains in scope until the end of `main()` (line 201). A clone `text_tx_t` is moved into the transcriber thread at line 153.

The shutdown sequence:
1. Ctrl+C fires → `running = false`
2. `capture.run()` returns → CPAL `stream` (local to `run()`) drops → callback drops `audio_tx_inner` → `audio_tx` parameter also drops → **all audio senders gone**
3. Transcriber `while let Ok(samples) = audio_rx.recv()` unblocks with `Err` → transcriber exits, dropping `text_tx_t`
4. `transcriber_handle.join()` returns successfully
5. `typer_handle.join()` is called — **DEADLOCK**: typer is blocked on `text_rx.recv()`, but `text_tx` (the original) is **still alive in main's scope** and won't be dropped until after `.join()` returns → neither can proceed

The comment on line 195 says "Channels will be dropped, threads will exit" — this is wrong. `text_tx` is NOT dropped before the join.

**Secondary issue — PTT threads block in `device.fetch_events()` (not a deadlock)**

PTT monitor threads in `src/ptt.rs` check `running` at the top of the loop but then call blocking `device.fetch_events()`. If Ctrl+C is pressed without a subsequent key event, the thread is stuck. However, since PTT threads are not joined, they don't block main from returning — once the `text_tx` deadlock is fixed, main exits and the OS kills all threads. This is still worth noting as a cleanup concern.

## Fix

**File:** [src/main.rs](src/main.rs) — lines 194–198

Add an explicit `drop(text_tx)` after joining the transcriber and before joining the typer:

```rust
info!("Stopping...");
drop(capture);           // drops CPAL stream → audio_tx_inner drops → audio_rx closes
let _ = transcriber_handle.join(); // transcriber exits (audio_rx closed), drops text_tx_t
drop(text_tx);           // ← ADD THIS: now all text_tx senders are gone
let _ = typer_handle.join(); // typer exits (text_rx closed)
```

This is a one-line fix at [src/main.rs:197](src/main.rs#L197).

## Why this works

After `transcriber_handle.join()`:
- `text_tx_t` (held by transcriber) is dropped — thread has exited
- `text_tx` (held by main) is dropped explicitly
- All senders gone → `text_rx.recv()` returns `Err` → typer loop exits → `typer_handle.join()` returns

## Verification

```bash
# Build
cargo build

# Run (needs a real model for full test, or use dry-run to test shutdown path)
./target/debug/whisper-type --dry-run --model /path/to/model.bin

# Press Ctrl+C — should see "Stopping..." then return to prompt within ~200ms

# Without model: test shutdown logic alone
cargo test
```

For a targeted shutdown test, a new integration test could be added that:
1. Spawns the crossbeam channels
2. Spawns transcriber and typer threads
3. Drops audio_tx and text_tx in order
4. Asserts both join handles complete within a timeout
