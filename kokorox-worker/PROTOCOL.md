# Native TTS worker

`kokorox-tts-worker` implements the existing spqx/PiBot stdio wire contract.
It is a separate executable, not an HTTP or WebSocket server. The launching
host resolves config and supplies explicit flags; this machine worker does
not read or create config files, download models, or auto-select voices.

## Build and launch

```bash
just worker-build
./target/release/kokorox-tts-worker --serve \
  --model /path/to/model.onnx --voices /path/to/voices.npz \
  --voice af_sarah --language en-us --speed 1.0
```

`--model-path`, `--data`, `--style`, and `--lan` are aliases for `--model`,
`--voices`, `--voice`, and `--language`. CUDA/CoreML builds use the respective
Cargo feature. Only native 24000 Hz output and v1.0-vocabulary models are
supported initially. No Chinese v1.1 auto-switching. Unknown voices/languages
fail rather than silently falling back; voice mixing is not enabled here.

## Frames

Every frame is a nine-byte header followed by its payload:
`u8 type`, `u32 request_id`, `u32 payload_length`, both integers little-endian.
Request id zero is reserved for startup/control. Do not reuse an id while it
is outstanding. Input payloads are capped at 1 MiB and at most eight requests
wait behind active synthesis; overflow returns an error.

| Direction | Type | Payload |
|-----------|------|---------|
| Input | speak (1) | Raw UTF-8 text, **not JSON** |
| Input | cancel (2) | Empty; header identifies the request |
| Input | shutdown (3) | Empty |
| Output | ready (1) | Empty; id zero; model/voice loaded |
| Output | audio_start (2) | Four-byte little-endian sample rate |
| Output | audio_chunk (3) | Signed PCM16 little-endian, mono |
| Output | audio_done (4) | Empty; terminal completion, including canceled work |
| Output | error (5) | UTF-8 message; terminal failure, no success completion |

Output chunks are at most `--blocksize` samples (default 4096). Both existing
engine stdout logging and native chatter are redirected to stderr before
initialization. Only frames use the preserved stdout pipe. Stderr contains
legacy diagnostics plus JSON load/TTFA/generation timing events; it is not an
exclusively JSON stream. RTF means wall time / generated audio duration here.

## Lifecycle

Kokoro produces audio per completed inference chunk, not per autoregressive
token. `TTSKoko::tts_stream_audio` emits those chunks without buffering the
whole utterance; the existing buffered method collects the same chunks.

An independent input reader processes cancel while synthesis is running.
Cancellation and output share one lock: after cancellation is observed, no
more audio frames for that request are written. Already-sent audio cannot be
recalled; playback/generation fencing remains the host's responsibility.
Unknown cancel ids are ignored, and canceled queued requests never synthesize.
The next request remains usable. A running ONNX kernel cannot be interrupted;
cancellation suppresses its output and stops further inference chunks.

EOF drains accepted requests. Explicit shutdown fences active output and drops
queued work, then exits after any currently running ONNX call returns.
Malformed/truncated/oversized input emits a control error and stops the worker.
A missing local asset emits startup error id zero and exits nonzero, without
ready. Malformed text and inference errors do not terminate the process.

## German exports

Validated locally with `Godelaune/Kokoro-82M-ONNX-German-Martin`:
`kokoro-martin.onnx`, `voices-martin.npz`, voice `martin`, language `de`.
Its token input is `tokens`, not `input_ids`; the loader now discovers either
name from ONNX metadata. Style and speed inputs keep the standard shapes.
The NPZ contains `martin` with shape `(510, 1, 256)` float32.

Legacy kokorox `.bin` voice packs are **NPZ archives**, not raw float32 files.
Crane's `voices/df_kerstin.bin` is raw style rows and needs wrapping into an
NPZ entry shaped `(rows, 1, 256)` before this worker can read it; that export
has not been tested here. File extensions alone do not establish compatibility.

Martin's graph contains random operators; independent runs are not
byte-identical. Its smoke test checks framing and non-silent audio, with
separate ASR verification for intelligibility. Do not claim deterministic
PCM parity for stochastic models. Models/audio remain outside git.

## Verification

```bash
just worker-test
KOKOROX_TEST_MODEL=/path/to/model.onnx \
KOKOROX_TEST_VOICES=/path/to/voices.npz \
KOKOROX_TEST_VOICE=af_sarah just worker-test-model
```

The ignored real-model test compares buffered, chunked, and worker PCM for
deterministic exports. Set `KOKOROX_TEST_LANGUAGE=de`, `KOKOROX_TEST_VOICE=martin`,
and `KOKOROX_TEST_STOCHASTIC=1` for Martin (audio smoke only). Optional
`KOKOROX_TEST_TEXT` selects a corpus and `KOKOROX_TEST_PCM_OUTPUT` saves PCM for
external ASR/listening. Unit tests exercise raw UTF-8 framing, rate payload,
PCM saturation, malformed input, errors, active/queued cancel, and EOF drain;
a non-ignored black-box test checks startup error framing in the real binary.

Only macOS CPU execution has been run here; Linux/Windows and GPU execution
are not yet demonstrated. Foxline adapter/Persona integration is separate
work (`fxl-8bht`, `fxl-00hv`); matching bytes alone is not proof of a complete
Foxline conversation.

## Licensing

This worker uses the existing espeak-linked engine. It does not change that
engine's distribution obligations or authorize relicensing. The README and
existing Cargo license metadata disagree; `kokorox-8j45` must audit source
provenance and transitive dependencies before any permissive-license claim.
A subprocess boundary is an architectural choice, not automatic legal clearance.
