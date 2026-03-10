use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GpuBackend {
    #[default]
    Auto,
    Cuda,
    Vulkan,
    Cpu,
}

/// Fully resolved backend — never Auto. Carries the device index.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedBackend {
    Cuda(u32),
    Vulkan(u32),
    Cpu,
}

/// Resolve the configured backend to a concrete choice.
/// `Auto` probes for NVIDIA via nvml, then falls back to Vulkan, then CPU.
pub fn detect_backend(backend: &GpuBackend, device: u32) -> ResolvedBackend {
    match backend {
        GpuBackend::Cpu => ResolvedBackend::Cpu,
        GpuBackend::Cuda => ResolvedBackend::Cuda(device),
        GpuBackend::Vulkan => ResolvedBackend::Vulkan(device),
        GpuBackend::Auto => probe_auto(device),
    }
}

fn probe_auto(device: u32) -> ResolvedBackend {
    // Try NVIDIA via NVML
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if nvml.device_count().unwrap_or(0) > 0 {
            tracing::debug!("auto: NVIDIA GPU detected via NVML, selecting CUDA backend");
            return ResolvedBackend::Cuda(device);
        }
    }
    // Try Vulkan
    let vulkan_devices = whisper_rs::vulkan::list_devices();
    if !vulkan_devices.is_empty() {
        tracing::debug!("auto: Vulkan devices found, selecting Vulkan backend");
        return ResolvedBackend::Vulkan(device);
    }
    tracing::debug!("auto: no GPU detected, falling back to CPU");
    ResolvedBackend::Cpu
}

/// Print available GPU devices to stdout.
/// Shows CUDA devices (via NVML) and Vulkan devices in separate sections.
/// Silently skips each section if the underlying library fails to initialize.
pub fn list_gpu_devices() {
    let mut any = false;

    // CUDA devices via NVML
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        let count = nvml.device_count().unwrap_or(0);
        if count > 0 {
            any = true;
            println!("CUDA devices (NVIDIA):");
            for i in 0..count {
                match nvml.device_by_index(i) {
                    Err(_) => println!("  {i}: (failed to query device)"),
                    Ok(dev) => {
                        let name = dev.name().unwrap_or_else(|_| "unknown".to_string());
                        match dev.memory_info() {
                            Ok(m) => println!(
                                "  {i}: {}  ({} MB total, {} MB free)",
                                name,
                                m.total / 1024 / 1024,
                                m.free / 1024 / 1024,
                            ),
                            Err(_) => println!("  {i}: {name}"),
                        }
                    }
                }
            }
        }
    }

    // Vulkan devices
    let vulkan_devices = whisper_rs::vulkan::list_devices();
    if !vulkan_devices.is_empty() {
        any = true;
        println!("Vulkan devices:");
        for dev in &vulkan_devices {
            println!(
                "  {}: {}  ({} MB total, {} MB free)",
                dev.id,
                dev.name,
                dev.vram.total / 1024 / 1024,
                dev.vram.free / 1024 / 1024,
            );
        }
    }

    if !any {
        println!("No GPU devices found.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_backend_default_is_auto() {
        assert_eq!(GpuBackend::default(), GpuBackend::Auto);
    }

    #[test]
    fn test_gpu_backend_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&GpuBackend::Auto).unwrap(),
            r#""auto""#
        );
        assert_eq!(
            serde_json::to_string(&GpuBackend::Cuda).unwrap(),
            r#""cuda""#
        );
        assert_eq!(
            serde_json::to_string(&GpuBackend::Vulkan).unwrap(),
            r#""vulkan""#
        );
        assert_eq!(serde_json::to_string(&GpuBackend::Cpu).unwrap(), r#""cpu""#);
    }

    #[test]
    fn test_gpu_backend_deserializes_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<GpuBackend>(r#""auto""#).unwrap(),
            GpuBackend::Auto
        );
        assert_eq!(
            serde_json::from_str::<GpuBackend>(r#""cuda""#).unwrap(),
            GpuBackend::Cuda
        );
        assert_eq!(
            serde_json::from_str::<GpuBackend>(r#""vulkan""#).unwrap(),
            GpuBackend::Vulkan
        );
        assert_eq!(
            serde_json::from_str::<GpuBackend>(r#""cpu""#).unwrap(),
            GpuBackend::Cpu
        );
    }

    #[test]
    fn test_gpu_backend_invalid_value_fails() {
        assert!(serde_json::from_str::<GpuBackend>(r#""GPU""#).is_err());
    }

    #[test]
    fn test_detect_cpu_returns_cpu() {
        assert_eq!(detect_backend(&GpuBackend::Cpu, 0), ResolvedBackend::Cpu);
    }

    #[test]
    fn test_detect_cuda_explicit_returns_cuda_with_device() {
        assert_eq!(
            detect_backend(&GpuBackend::Cuda, 2),
            ResolvedBackend::Cuda(2)
        );
    }

    #[test]
    fn test_detect_vulkan_explicit_returns_vulkan_with_device() {
        assert_eq!(
            detect_backend(&GpuBackend::Vulkan, 1),
            ResolvedBackend::Vulkan(1)
        );
    }
}
