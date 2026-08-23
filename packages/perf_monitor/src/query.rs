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
    profiler_opt: Option<
        &std::sync::Mutex<
            Option<std::sync::Arc<std::sync::Mutex<sniffer_core::profiler::FrameProfiler>>>,
        >,
    >,
) -> Result<String, PackageError> {
    match method {
        "getCpuUsage" => Ok(format!(r#"{{"ok":true,"cpu_usage":{cpu:.1}}}"#)),

        "getMemoryUsage" => Ok(format!(r#"{{"ok":true,"mem_mb":{mem_mb:.0}}}"#)),

        "getAllMetrics" => Ok(format!(
            r#"{{"ok":true,"cpu_usage":{cpu:.1},"mem_mb":{mem_mb:.0},"fps":{fps:.1}}}"#,
        )),

        "getDevKitTelemetry" => {
            let (p_fps, avg_ms, cpu_ms, gpu_ms, swap_ms, pointers, gesture, target, tx, ty) =
                if let Some(p_lock) = profiler_opt
                    && let Ok(guard) = p_lock.lock()
                    && let Some(ref prof_arc) = *guard
                    && let Ok(prof) = prof_arc.lock()
                {
                    (
                        prof.current_fps(),
                        prof.average_frame_time_ms(),
                        prof.avg_cpu_ms(),
                        prof.avg_gpu_ms(),
                        prof.avg_swap_ms(),
                        prof.touch_telemetry.active_pointers,
                        prof.touch_telemetry.gesture.clone(),
                        prof.touch_telemetry.target_element.clone(),
                        prof.touch_telemetry.touch_x,
                        prof.touch_telemetry.touch_y,
                    )
                } else {
                    (
                        fps,
                        if fps > 0.0 { 1000.0 / fps } else { 0.0 },
                        0.0,
                        0.0,
                        0.0,
                        0,
                        "IDLE".to_string(),
                        "None".to_string(),
                        0.0,
                        0.0,
                    )
                };

            Ok(format!(
                r#"{{"ok":true,"fps":{p_fps:.1},"frame_time_ms":{avg_ms:.1},"cpu_ms":{cpu_ms:.1},"gpu_ms":{gpu_ms:.1},"swap_ms":{swap_ms:.1},"cpu_usage":{cpu:.1},"mem_mb":{mem_mb:.0},"touch":{{"active_pointers":{pointers},"gesture":"{gesture}","target":"{target}","touch_x":{tx:.1},"touch_y":{ty:.1}}}}}"#
            ))
        }

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
