# kokorox-websocket

WebSocket server for real-time TTS streaming with kokorox.

## Features

- Real-time text-to-speech via WebSocket
- Sentence-level audio streaming
- **Streaming mode** for low-latency incremental TTS (perfect for LLM output)
- Voice and speed control
- Automatic language detection

## Usage

```bash
# Start the WebSocket server
koko websocket --port 8766

# With dynamic model switching (for multi-language)
koko websocket --port 8766 --dynamic
```

## Protocol

All messages are JSON. Audio chunks are base64-encoded WAV.

### Commands

#### List Voices

```json
{"command": "list_voices"}
```

Response:
```json
{"type": "voices", "voices": ["af_heart", "af_sky", ...], "voice": "af_heart"}
```

#### Set Voice

```json
{"command": "set_voice", "voice": "af_sky"}
```

Response:
```json
{"type": "voice_changed"}
```

#### Set Speed

```json
{"command": "set_speed", "speed": 1.2}
```

Response:
```json
{"type": "speed_changed"}
```

#### Synthesize (One-Shot)

Send text, receive audio chunks:

```json
{"command": "synthesize", "text": "Hello, world!"}
```

Responses:
```json
{"type": "synthesis_started"}
{"type": "audio_chunk", "chunk": "<base64 WAV>", "index": 0, "total": 2, "sample_rate": 24000}
{"type": "audio_chunk", "chunk": "<base64 WAV>", "index": 1, "total": 2, "sample_rate": 24000}
{"type": "synthesis_completed"}
```

### Streaming Mode (Low-Latency)

For streaming LLM output, use streaming mode. Text is buffered and synthesized sentence-by-sentence as it arrives, providing minimal latency.

#### Start Stream

```json
{"command": "stream_start", "voice": "af_heart", "speed": 1.0}
```

Response:
```json
{"type": "stream_started", "stream_id": "uuid-here"}
```

#### Append Text

Send text incrementally as it streams from the LLM:

```json
{"command": "stream_append", "stream_id": "uuid-here", "text": "Hello, "}
```

```json
{"command": "stream_append", "stream_id": "uuid-here", "text": "this is a test. "}
```

When a complete sentence is detected, audio is sent immediately:

```json
{"type": "stream_chunk", "stream_id": "uuid-here", "chunk": "<base64 WAV>", "index": 0, "sample_rate": 24000}
```

#### End Stream

Flush remaining text and close the stream:

```json
{"command": "stream_end", "stream_id": "uuid-here"}
```

Response:
```json
{"type": "stream_ended", "stream_id": "uuid-here", "total_chunks": 5}
```

#### Cancel Stream

Abort without flushing:

```json
{"command": "stream_cancel", "stream_id": "uuid-here"}
```

Response:
```json
{"type": "stream_cancelled", "stream_id": "uuid-here"}
```

### Streaming Mode Benefits

1. **Low latency**: Audio starts playing as soon as the first sentence is complete
2. **Seamless playback**: All chunks belong to the same stream, no gaps between sentences
3. **LLM integration**: Perfect for streaming responses from ChatGPT, Claude, etc.
4. **Backpressure handling**: Client controls when to send more text

### Example: LLM Integration

```javascript
// Start stream when assistant message begins
const streamId = await tts.streamStart();

// As LLM tokens arrive, append them
for await (const token of llmStream) {
  tts.streamAppend(token);
}

// End stream when message is complete
tts.streamEnd();
```

## Audio Format

All audio chunks are:
- Format: WAV (PCM)
- Sample rate: 24000 Hz
- Channels: 1 (mono)
- Bit depth: 16-bit signed integer

## Error Handling

```json
{"type": "error", "message": "Description of error"}
```

## License

Apache-2.0
