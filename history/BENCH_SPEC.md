# kokorox Benchmark Specification

## Overview

Add a `bench` subcommand to kokorox/koko for standardized TTS (Text-to-Speech) benchmarking as part of the OwlBench suite.

**Issue**: `owlbench-cgh.2` - Add bench subcommand to kokorox (TTS benchmarking)

## Command Interface

```bash
koko bench [OPTIONS]
koko bench --output json > results.json
koko bench --style af_heart --runs 5
koko bench --all-styles  # benchmark all available voices
```

## Required Metrics

| Metric | Description | Unit |
|--------|-------------|------|
| `rtf` | Real-Time Factor (audio_duration / generation_time) | ratio |
| `first_byte_latency_ms` | Time to first audio byte (streaming mode) | ms |
| `chars_per_second` | Characters processed per second | chars/sec |
| `latency_p50_ms` | 50th percentile generation latency | ms |
| `latency_p99_ms` | 99th percentile generation latency | ms |
| `memory_peak_mb` | Peak memory usage during inference | MB |
| `samples_per_second` | Audio samples generated per second | samples/sec |

## Test Sentences

Use standardized test sentences of varying lengths:

```
Short  (~10 chars):  "Hello world"
Medium (~50 chars):  "The quick brown fox jumps over the lazy dog."
Long   (~200 chars): "In a hole in the ground there lived a hobbit. Not a nasty, dirty, wet hole, filled with the ends of worms and an oozy smell, nor yet a dry, bare, sandy hole."
```

Include multi-language tests if benchmarking language support.

## JSON Output Schema

```json
{
  "benchmark": "koko-tts",
  "version": "0.1.0",
  "timestamp": "2025-12-11T16:00:00Z",
  "hardware": {
    "cpu": "Apple M3 Max",
    "gpu": "Apple M3 Max (Metal)",
    "memory_gb": 64
  },
  "config": {
    "model": "kokoro-v1.0.onnx",
    "style": "af_heart",
    "language": "en-us",
    "runs": 5
  },
  "results": {
    "rtf": 0.08,
    "first_byte_latency_ms": 45,
    "chars_per_second": 850,
    "latency_p50_ms": 120,
    "latency_p99_ms": 280,
    "memory_peak_mb": 420,
    "samples_per_second": 192000
  }
}
```

## Implementation Notes

1. **Warm-up run**: Discard first run to exclude model/voice loading
2. **Multiple runs**: Default 5 runs, report median and p99
3. **Streaming vs batch**: Benchmark both modes if applicable
4. **Voice comparison**: Optional `--all-styles` to compare voice performance
5. **Output format**: Measure WAV generation (not MP3 encoding overhead)

## Integration with OwlBench

Results will be collected by `owlbench collect` command and aggregated into composite Audio Score.

```bash
# OwlBench will call:
koko bench --output json > /tmp/koko_bench.json
```
