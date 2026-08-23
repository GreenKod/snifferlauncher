use super::constants::{POLL_INTERVAL_MAX_MS, POLL_INTERVAL_MIN_MS};
use sniffer_pkg::error::PackageError;
use std::sync::atomic::{AtomicU64, Ordering};

pub fn handle_query(
    method: &str,
    payload: &str,
    cpu: f32,
    mem_mb: f32,
    fps: f32,
    poll_interval_ms: &AtomicU64,
) -> Result<String, PackageError> {
    match method {
        "getCpuUsage" => Ok(format!(r#"{{"ok":true,"cpu_usage":{cpu:.1}}}"#)),

        "getMemoryUsage" => Ok(format!(r#"{{"ok":true,"mem_mb":{mem_mb:.0}}}"#)),

        "getAllMetrics" => Ok(format!(
            r#"{{"ok":true,"cpu_usage":{cpu:.1},"mem_mb":{mem_mb:.0},"fps":{fps:.1}}}"#,
        )),

        "setPollingInterval" => {
            let val: serde_json::Value = serde_json::from_str(payload)
                .map_err(|e| PackageError::InvalidDescriptor(e.to_string()))?;

            let requested = val["interval_ms"].as_u64().ok_or_else(|| {
                PackageError::InvalidDescriptor(
                    "missing or invalid 'interval_ms' field (expected u64)".into(),
                )
            })?;

            let clamped = requested.clamp(POLL_INTERVAL_MIN_MS, POLL_INTERVAL_MAX_MS);
            poll_interval_ms.store(clamped, Ordering::Relaxed);

            Ok(format!(r#"{{"ok":true,"interval_ms":{clamped}}}"#))
        }

        _ => Err(PackageError::UnsupportedMethod(method.into())),
    }
}
