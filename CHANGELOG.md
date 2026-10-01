# Changelog

## Unreleased

- Corrected all five Cargo package declarations to inherit the current `GPL-3.0-only` workspace license, matching the root license and native worker. Corrected stale WebSocket license documentation and added positive/negative metadata-consistency checks.
- Recorded a preliminary permissive-transition audit: eSpeak's embedded GPL code is not covered by its Rust wrapper's MIT metadata; unconditional MP3 support also brings in static LGPL-3.0 LAME. Crane's MIT G2P code and CC BY-SA German dictionary require separate treatment.
- Set MIT as the target for source the project can lawfully license that way, retaining inherited notices. No dependency removal, MIT relicensing or WASM compatibility is claimed by this metadata correction; implementation and rights/quality gates remain tracked in `kokorox-8j45`.
