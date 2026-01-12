# kokorox - fast Kokoro TTS in Rust

Rust implementation of the [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M) text-to-speech model. Small model (87M parameters), high quality output, very fast inference.

## Features

- Multi-language: English, Chinese, Japanese, Spanish, French, and more via espeak-ng
- Voice style mixing (e.g., `af_sky.4+af_nicole.5`)
- OpenAI-compatible API server
- Streaming and pipe modes for LLM integration
- Automatic language detection

## Quick Start

```bash
# Install (macOS)
brew install byteowlz/tap/koko

# Or download from GitHub Releases
# https://github.com/byteowlz/kokorox/releases

# Generate speech
koko text "Hello, this is a test"

# Output: tmp/output.wav
```

## Installation

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/byteowlz/kokorox/releases) for Linux, macOS, and Windows.

### From Source

Requires ONNX runtime and espeak-ng:

```bash
# macOS
brew install espeak-ng

# Ubuntu/Debian
sudo apt-get install espeak-ng libespeak-ng-dev
```

Build:

```bash
git clone https://github.com/byteowlz/kokorox.git
cd kokorox
pip install -r scripts/requirements.txt
python scripts/download_voices.py --all
cargo build --release
```

### ONNX Runtime (Linux with NVIDIA GPU)

```bash
tar -xzf onnxruntime-linux-x64-gpu-1.22.0.tgz
sudo cp -a onnxruntime-linux-x64-gpu-1.22.0/include /usr/local/
sudo cp -a onnxruntime-linux-x64-gpu-1.22.0/lib /usr/local/
sudo ldconfig
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

## Usage

### Basic

```bash
koko text "Hello, world!" -o greeting.wav
koko file poem.txt                          # One wav per line
```

### Multi-language

```bash
koko text "Hola, mundo!" --lan es
koko text "你好，世界!" --lan zh
koko -a text "Bonjour!"                     # Auto-detect language
```

### Voice Styles

```bash
koko voices                                 # List available voices
koko voices --language en --gender female   # Filter voices
koko text "Hello" --style af_sky
koko text "Hello" --style af_sky.4+af_nicole.5  # Mix styles
```

### Pipe Mode (LLM Integration)

```bash
ollama run llama3 "Tell me a story" | koko pipe
ollama run llama3 "Explain physics" | koko pipe --silent -o output.wav
```

### OpenAI-Compatible Server

```bash
koko openai --ip 0.0.0.0 --port 3000
```

```bash
curl -X POST http://localhost:3000/v1/audio/speech \
  -H "Content-Type: application/json" \
  -d '{"model": "kokoro", "input": "Hello!", "voice": "af_sky"}' \
  -o hello.wav

curl http://localhost:3000/v1/audio/voices           # List voice IDs
curl http://localhost:3000/v1/audio/voices/detailed  # Voice metadata
```

### Streaming

```bash
koko stream > output.wav
# Type text, press Enter. Ctrl+D to exit.
```

### Docker

```bash
docker build -t kokorox .
docker run -v ./tmp:/app/tmp kokorox text "Hello from docker!" -o tmp/hello.wav
docker run -p 3000:3000 kokorox openai --ip 0.0.0.0 --port 3000
```

## Debugging

```bash
koko text "Text here" --verbose              # Detailed processing logs
koko text "Accénted" --debug-accents         # Character-by-character analysis
```

## Additional Voices

The default installation includes standard voices. More voices (54 total across 8 languages) can be converted from Hugging Face:

```bash
python scripts/convert_pt_voices.py --all
koko -d data/voices-custom.bin text "Hello" --style en_sarah
```

## License

GPL 3.0 due to use of the espeak-rs-sys crate which statically links espeak-ng
