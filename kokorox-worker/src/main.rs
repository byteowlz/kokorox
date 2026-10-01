//! Explicit, config-file-free machine worker. CLI/config policy belongs to the
//! launching host, just as for spqx-tts-worker.
mod protocol;
mod server;

use std::io::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use filedescriptor::{FileDescriptor, StdioDescriptor};
use kokorox::tts::koko::TTSKoko;

#[derive(Debug, Parser)]
#[command(about = "Persistent Kokoro worker using the spqx binary stdio contract")]
struct Args {
    /// Accepted for compatibility with hosts launching spqx --serve.
    #[arg(long)]
    serve: bool,
    /// Local ONNX model; no implicit downloads in a machine worker.
    #[arg(long = "model", alias = "model-path")]
    model: PathBuf,
    /// Named voice NPZ archive (.npz or legacy .bin); not raw float32 rows.
    #[arg(long = "voices", alias = "data")]
    voices: PathBuf,
    /// Exact voice id present in the archive (no silent fallback).
    #[arg(long = "voice", alias = "style")]
    voice: String,
    #[arg(long = "language", alias = "lan", default_value = "en-us")]
    language: String,
    #[arg(long, default_value_t = 1.0)]
    speed: f32,
    /// Kokoro natively outputs 24000 Hz; other rates are not supported yet.
    #[arg(long, default_value_t = 24000)]
    output_sample_rate: u32,
    /// Maximum PCM samples per audio_chunk frame.
    #[arg(long, default_value_t = 4096)]
    blocksize: usize,
}

fn main() {
    if let Err(error) = run(Args::parse()) {
        eprintln!("kokorox worker: {error}");
        std::process::exit(1);
    }
}

fn run(mut args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // Preserve the original stdout pipe for protocol frames, and redirect all
    // existing Rust/native engine chatter before initialization. This helper
    // supports Unix and Windows without platform-specific unsafe code here.
    io::stdout().flush()?;
    let mut writer = FileDescriptor::redirect_stdio(&io::stderr(), StdioDescriptor::Stdout)?;
    let started = Instant::now();
    let engine = match catch_unwind(AssertUnwindSafe(|| load(&mut args))) {
        Ok(Ok(engine)) => engine,
        result => {
            let message = match result {
                Ok(Err(error)) => error.to_string(),
                _ => "engine initialization panicked; see stderr".to_string(),
            };
            protocol::write_frame(&mut writer, protocol::ERROR, 0, message.as_bytes())?;
            return Err(io::Error::other(message).into());
        }
    };
    eprintln!(
        "{{\"type\":\"server_ready\",\"backend\":\"kokorox_onnx\",\"load_seconds\":{:.3}}}",
        started.elapsed().as_secs_f64()
    );
    server::serve(
        io::stdin(),
        writer,
        engine.sample_rate(),
        args.blocksize,
        |text, emit| {
            let started = Instant::now();
            let mut first_audio = true;
            let mut sample_count = 0;
            let result = catch_unwind(AssertUnwindSafe(|| {
                engine.tts_stream_audio(
            text, &args.language, &args.voice, args.speed, None, false, true, false,
            &mut |samples| {
                let keep_going = emit(samples);
                if keep_going {
                    if first_audio {
                        eprintln!("{{\"type\":\"ttfa\",\"seconds\":{:.3},\"backend\":\"kokorox_onnx\"}}", started.elapsed().as_secs_f64());
                        first_audio = false;
                    }
                    sample_count += samples.len();
                }
                keep_going
            },
        ).map_err(|error| error.to_string())
            }));
            let audio_seconds = sample_count as f64 / engine.sample_rate() as f64;
            let wall = started.elapsed().as_secs_f64();
            let rtf = if audio_seconds > 0.0 {
                format!("{:.3}", wall / audio_seconds)
            } else {
                "null".into()
            };
            eprintln!("{{\"type\":\"generated\",\"wall_seconds\":{wall:.3},\"audio_seconds\":{audio_seconds:.3},\"rtf\":{rtf}}}");
            result.unwrap_or_else(|_| Err("inference panicked; see stderr".into()))
        },
    )?;
    Ok(())
}

fn load(args: &mut Args) -> Result<TTSKoko, Box<dyn std::error::Error>> {
    args.language = kokorox::tts::phonemizer::normalize_language_code(&args.language)
        .ok_or("unsupported language (no automatic language/voice fallback)")?;
    if args.output_sample_rate != 24000 || args.blocksize == 0 || args.blocksize > 131072 {
        return Err("require 24000 Hz and blocksize 1..=131072".into());
    }
    if !args.speed.is_finite() || args.speed <= 0.0 {
        return Err("speed must be finite and positive".into());
    }
    for path in [&args.model, &args.voices] {
        if !path.is_file() {
            return Err(format!("local asset does not exist: {}", path.display()).into());
        }
    }
    // Engine uses the Kokoro v1.0 IPA vocabulary; Chinese v1.1 is a separate
    // model/vocab pair and not silently auto-selected for a pinned worker.
    if args.language.starts_with("zh") {
        return Err(
            "this worker currently requires a v1.0-vocabulary model; Chinese v1.1 is unsupported"
                .into(),
        );
    }
    kokorox::onn::init_ort(None).map_err(io::Error::other)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let model = args.model.to_str().ok_or("model path is not UTF-8")?;
    let voices = args.voices.to_str().ok_or("voices path is not UTF-8")?;
    let engine = runtime.block_on(TTSKoko::from_paths(model, voices));
    if !engine.get_available_voices().contains(&args.voice) {
        return Err(format!("voice '{}' is not in the archive", args.voice).into());
    }
    // af_heart is treated as a default by the existing core for non-English
    // languages, which can silently substitute another voice. Reject that
    // combination until the core's explicit-voice semantics are separated.
    if args.voice == "af_heart" && !args.language.starts_with("en") {
        return Err(
            "af_heart is a core default for non-English; choose an explicit language-matched voice"
                .into(),
        );
    }
    Ok(engine)
}
