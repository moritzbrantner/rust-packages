use video_analysis_core::Result;

use crate::{setup_error, NativeDevicePreference};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedNativeDevice {
    Cpu,
    #[cfg(feature = "cuda")]
    Cuda(usize),
}

pub(crate) fn resolve_native_device(
    preference: NativeDevicePreference,
) -> Result<ResolvedNativeDevice> {
    match preference {
        NativeDevicePreference::Cpu => Ok(ResolvedNativeDevice::Cpu),
        NativeDevicePreference::Cuda => resolve_requested_cuda(),
        NativeDevicePreference::Auto => resolve_auto_device(),
    }
}

#[cfg(not(feature = "cuda"))]
fn resolve_requested_cuda() -> Result<ResolvedNativeDevice> {
    Err(setup_error(
        "CUDA was requested but the binary lacks the `cuda` feature",
    ))
}

#[cfg(feature = "cuda")]
fn resolve_requested_cuda() -> Result<ResolvedNativeDevice> {
    candle_core::Device::new_cuda(0)
        .map(|_| ResolvedNativeDevice::Cuda(0))
        .map_err(|error| {
            setup_error(format!(
                "CUDA was requested but no CUDA device is available: {error}"
            ))
        })
}

#[cfg(not(feature = "cuda"))]
fn resolve_auto_device() -> Result<ResolvedNativeDevice> {
    Ok(ResolvedNativeDevice::Cpu)
}

#[cfg(feature = "cuda")]
fn resolve_auto_device() -> Result<ResolvedNativeDevice> {
    match candle_core::Device::new_cuda(0) {
        Ok(_) => Ok(ResolvedNativeDevice::Cuda(0)),
        Err(_) => Ok(ResolvedNativeDevice::Cpu),
    }
}
