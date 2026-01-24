pub mod ort_base;
pub mod ort_koko;

/// Initialize ONNX Runtime. Must be called before using any ort functionality
/// when the `load-dynamic` feature is enabled.
///
/// For CUDA builds with `load-dynamic`, you need to either:
/// 1. Set the `ORT_DYLIB_PATH` environment variable to point to `libonnxruntime.so`
/// 2. Call this function with a path to the library
///
/// Example:
/// ```no_run
/// // Via environment variable (recommended):
/// // export ORT_DYLIB_PATH=/path/to/libonnxruntime.so
///
/// // Or programmatically:
/// kokorox::onn::init_ort(Some("/path/to/libonnxruntime.so")).unwrap();
/// ```
#[cfg(feature = "cuda")]
pub fn init_ort(dylib_path: Option<&str>) -> Result<(), String> {
    if let Some(path) = dylib_path {
        let builder =
            ort::init_from(path).map_err(|e| format!("Failed to load ort from {}: {}", path, e))?;
        if !builder.commit() {
            return Err("Failed to commit ort environment (already initialized?)".to_string());
        }
    } else if std::env::var("ORT_DYLIB_PATH").is_ok() {
        // Environment variable is set, ort will use it automatically
        if !ort::init().commit() {
            // This is fine - environment was already initialized
            eprintln!("Note: ONNX Runtime environment was already initialized");
        }
    } else {
        return Err(
            "CUDA feature requires ORT_DYLIB_PATH environment variable or explicit path. \
             Download ONNX Runtime GPU from https://github.com/microsoft/onnxruntime/releases \
             and set ORT_DYLIB_PATH to the path of libonnxruntime.so"
                .to_string(),
        );
    }
    Ok(())
}

/// Initialize ONNX Runtime for non-CUDA builds (uses bundled binaries)
#[cfg(not(feature = "cuda"))]
pub fn init_ort(_dylib_path: Option<&str>) -> Result<(), String> {
    // Non-CUDA builds use the bundled ONNX Runtime, no explicit init needed
    Ok(())
}
