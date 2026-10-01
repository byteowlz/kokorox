# Worker verification — 2026-10-01

Environment: macOS, Apple M2 Ultra, ONNX CPU execution provider. This is a
native worker milestone (`kokorox-7ngw`), not a Foxline conversation or a
cross-platform performance claim.

## Passing checks

- `cargo check --workspace`
- `cargo test -p kokorox --lib`: 27 tests
- `just worker-test`: 10 protocol/lifecycle unit tests and one black-box
  startup test; the optional real-model test is ignored in this default run.
- `cargo clippy -p kokorox-worker --tests --no-deps -- -D warnings`
- `just worker-build`: release binary built
- Release real-model test with cached Kokoro v1.0, `af_sarah`, `en-us`:
  buffered core, callback core and binary-worker PCM matched byte-for-byte.
  The repeated default corpus crossed the core's inference-chunk boundary,
  not just the worker's PCM framing boundary (33.35 seconds of audio).
- Release real-model smoke with Martin/de: raw-text request, empty ready,
  sample rate in audio_start, non-silent PCM, completion and clean shutdown.
  Independent PCM equality is deliberately not asserted: Martin's graph
  contains RandomUniformLike and RandomNormalLike operators.
- `git diff --check`

The generic fake-synthesis lifecycle tests exercise active and queued cancel,
next-request recovery, shutdown fencing, EOF draining, bounded queuing,
unknown cancels, terminal failures, truncated/oversized frames and PCM clipping.
The real executable startup test catches stdout contamination independently of
its internal framing helpers.

## German intelligibility check

Input:

> Guten Morgen. Heute testen wir eine deutsche Stimme mit dem neuen Sprachdienst.

`trnscrb run --language de --transcribe-only` (Whisper backend) on worker PCM,
converted to 24000 Hz mono WAV, returned:

> Guten Morgen, heute testen wir eine deutsche Stimme mit dem neuen Sprachdienst.

Lexical content matched; punctuation differed. This is one ASR round-trip,
not human listening approval or a general quality benchmark. Generated WAV,
PCM, models and raw logs remain in local caches or temporary storage, not git.

Model: `Godelaune/Kokoro-82M-ONNX-German-Martin`, inspected revision
`a1cba7fbf0e72fbae38f0a3a48ce0dc8e6077804`.

- ONNX SHA256: `c302f1d8bc7adf40a842cb550e18c39a5026bdb1afdd29dbb700b501cb49276b`
- NPZ SHA256: `5b9c8553398d7abf67498ce500c186cefaa7b68fed3e3d415da5380670105acd`
- NPZ entry: `martin`, float32 `(510, 1, 256)`.
- Inputs: `tokens` int64 `(1, N)`, `style` float32 `(1, 256)`, `speed` float32 `(1)`.
  Discovering `tokens` rather than hardcoding `input_ids` fixed the initial
  reproducible German inference failure. Both export variants are exercised
  by the optional real-model test configurations.

## Review and gaps

ripwire reports unchanged `TTSKoko::tts_raw_audio` arity and no incompatible
call sites. Its quality delta has no pre-existing-worse gate, but it flags the
large existing preprocessing body moved into `tts_stream_audio`, its inherited
high arity/complexity, plus test-registration false positives and new callback
nesting. These are disclosed, not interpreted as a debt-free report. A broader
preprocessing refactor is deliberately not mixed into this worker change.

Linux/Windows, CUDA/CoreML, physical playback and full gateway barge-in have
not been demonstrated. Crane's raw voice file needs NPZ conversion and is not
validated here. Per-request voice changes, resampling and Chinese v1.1 model
switching are not implemented. HTTP convergence, Persona mapping and licensing
cleanup remain separate issues.
