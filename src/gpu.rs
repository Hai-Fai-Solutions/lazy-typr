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
}
