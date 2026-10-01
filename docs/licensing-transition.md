# Licensing transition: current GPL build to a permissive engine

Status: preliminary audit, not relicensing or distribution clearance. Tracked in `kokorox-8j45`. The requested target is MIT for code the project can lawfully offer under MIT, while retaining applicable third-party licenses/notices. No MIT or browser/WASM release is claimed yet.

## Current metadata correction

All five workspace packages inherit `GPL-3.0-only` from `[workspace.package]`, matching the current root `LICENSE` and existing native worker declaration. This corrects stale Apache-2.0 manifests and WebSocket documentation. It does not revoke historical permissive grants or imply that every third-party source file is exclusively GPL.

```bash
just license-metadata-check
just license-metadata-test
just check
just worker-test
```

The metadata checker compares resolved workspace declarations with the root license signature; its fixtures reject stale members/root mismatches. It is **not** a dependency/provenance audit and must not be used to authorize a future MIT release.

## Source history

- Initial commit `4da5f09` has no Cargo license field. Do not infer a grant from public availability alone.
- `2586d28` introduced an **Apache-2.0** root license on 2025-03-23. The recorded pre-GPL license was not MIT.
- `72b36e0` replaced that root license with GPLv3 on 2026-01-12; package declarations remained Apache-2.0. This history explains the mismatch, not a blanket right to relicense all source.
- Post-change commits currently use maintainer identities. Git authorship is an audit lead, not proof of copyright ownership or a replacement for reviewing copied code and contributor permissions.
- `kokorox/src/tts/chinese/tone_sandhi.rs` explicitly records adaptation from Misaki/PaddleSpeech. Identify exact upstream revisions and retain required licenses/copyright notices before a permissive release. Other inherited and adapted code needs the same review.

Recover applicable original permissive grants, identify GPL-only contributions requiring permission/replacement, and record authorization for maintainer-owned changes. Apache/BSD/MIT components keep their notices even if the project's own new code is MIT; do not replace third-party licenses with a blanket MIT label.

## Dependency findings

Inspection used the locked Cargo graph and downloaded crate sources. This is not a certified artifact SBOM; platform/features and native/vendor code need separate analysis.

| Component | Observation | Transition action |
| --- | --- | --- |
| `espeak-rs` / `espeak-rs-sys` 0.1.9 | Both declare MIT metadata, but sys embeds/builds eSpeak-ng; its public header grants GPLv3-or-later and the build defaults to static linking. All five packages reach it through `kokorox`. | Remove actual code/data dependency; wrapper metadata is not clearance. Switching to dynamic linking is not a GPL exemption. |
| `mp3lame-encoder` 0.2.1 / `mp3lame-sys` 0.1.9 | Declare LGPL-3.0; sys build emits `static=mp3lame`. MP3 helpers are unconditional in core, so even the PCM-only worker reaches LAME. | Keep out of a permissive-only inference/browser profile; remove/replace or isolate optional encoding with accurate obligations. LGPL does not itself prohibit MIT application code, but adds distribution/relinking obligations. |
| `option-ext`, Symphonia family | MPL-2.0 appears in the inventory. | Inspect actual target reachability and file-level obligations; MPL is not GPL and does not automatically relicense the whole application. |
| `jpreprocess` family / NAIST dictionary | Declared BSD-3-Clause metadata. | Inspect bundled dictionary notices/data provenance, not just wrapper fields. |
| ONNX Runtime and other native/system components | Rust ORT wrappers declare MIT OR Apache-2.0; Linux build tooling also probes sonic/pcaudio. | Review actual runtime/provider/system binaries and notices for each artifact. |
| Model/voice assets | Separate artifacts with separate terms. | Pin model/voice pairs and their licenses; code relicensing does not relicense weights or training data. |

Reproduce dependency reachability with `cargo tree --locked -i espeak-rs-sys` and `cargo tree --locked -i mp3lame-sys`. An all-green Cargo metadata license scan would miss the embedded eSpeak-ng issue.

## Espeak-free frontend candidate

Inspected Crane revision `2ee897a08e7b1df1c7cd2beca2aedd5dfe2e774d`:

- [Root MIT license](https://github.com/lucasjinreal/Crane/blob/2ee897a08e7b1df1c7cd2beca2aedd5dfe2e774d/LICENSE) and explicit MIT SPDX headers on German G2P/lexicon source.
- [G2P documentation](https://github.com/lucasjinreal/Crane/blob/2ee897a08e7b1df1c7cd2beca2aedd5dfe2e774d/crane-core/src/models/g2p/README.md) describes dictionary lookup, compound handling and handwritten fallback rules. Prefer a small audited extraction/adaptation, not a dependency on the entire inference framework.
- **German dictionary data is CC BY-SA 4.0**, extracted from Wiktionary. MIT code does not make this data MIT. Keep code/data licensing distinct, inspect the exact asset provenance and decide whether separately licensed share-alike data is acceptable; otherwise use a suitably licensed alternative or evaluate rule-only quality.
- Crane documents its particular English dictionary as MIT. Verify the pinned converted asset and provenance before copying it; do not generalize that license to every language in the same dataset.
- German dictionary IPA is not directly model-ready: stress placement, affricates and diphthongs require a model-specific adapter. Crane explicitly documents this mismatch. The Martin model's existing eSpeak-generated audio proof does not validate a replacement frontend.

Do not copy eSpeak implementation/dictionary tables into the new frontend, or bulk-generate a supposedly MIT replacement lexicon from them without a provenance review. Small validation corpora and implementation-free phoneme conventions are a different question from redistributing a dictionary.

## Release gates

1. Establish retained-source grants, contributor authorization and third-party notices. Keep unresolved ownership records private/local; do not commit contributor contact lists.
2. Prototype independently licensed English/German G2P and model-specific IPA normalization behind the existing frontend seam. Inventory current languages first; unsupported languages must fail explicitly, not become silent regressions or automatic English substitutions.
3. Remove eSpeak from dependency resolution/build artifacts. Build on macOS/Linux without eSpeak development libraries; validate native worker framing/cancellation unchanged.
4. Remove copyleft-only dependencies from the intended permissive/browser profile, including unnecessary MP3 encoding. If optional LGPL encoders remain elsewhere, document them accurately and preserve API behavior or explicitly reject unsupported formats.
5. Run a fixed corpus of German/English plain speech, names, compounds, abbreviations, numbers, dates, units and currencies. Check vocabulary/stress output, synthesize real audio, measure ASR errors/TTFA/RTF against the current frontend and obtain listening approval. Do not select solely on dictionary CER or a model card.
6. Build/review the actual WASM, JS glue, dependency/data inventory and matching source/build instructions. Native `ort`/eSpeak builds are not evidence of a browser-ready port.
7. Only after those gates, update root license and package declarations to the authorized permissive terms and publish version-matched notices. Keep earlier releases' licenses/source available.

Removing eSpeak is an engineering milestone, not automatic legal clearance. A subprocess, worker, iframe or separate repository is also not a blanket license exemption. See the [GPL FAQ on separate/combined programs](https://www.gnu.org/licenses/gpl-faq.html#MereAggregation).
