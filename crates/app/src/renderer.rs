//! What the window is drawn with. eframe's own setup opens Vulkan, DX12 and OpenGL together,
//! and each keeps the driver of every GPU loaded for the life of the process: on Windows one
//! API is opened when a GPU answers it, and wgpu is asked to reserve memory by small blocks.

use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
use eframe::wgpu;

/// The `e2e` flavour draws on the software adapter whatever the machine offers, as the runners
/// do (WARP, lavapipe): eframe reads no variable to choose one. `DEVNOTES_E2E_GPU` leaves the
/// choice to eframe, to measure what a person's window costs.
#[cfg(feature = "e2e")]
pub(crate) fn options() -> WgpuConfiguration {
    if std::env::var_os("DEVNOTES_E2E_GPU").is_some() {
        return WgpuConfiguration::default();
    }
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.native_adapter_selector = Some(std::sync::Arc::new(|adapters, surface| {
        let draws_here = |adapter: &&wgpu::Adapter| {
            surface.is_none_or(|surface| adapter.is_surface_supported(surface))
        };
        adapters
            .iter()
            .filter(draws_here)
            .find(|adapter| adapter.get_info().device_type == wgpu::DeviceType::Cpu)
            .or_else(|| adapters.iter().find(draws_here))
            .cloned()
            .ok_or_else(|| "no adapter draws on this window".to_string())
    }));
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

#[cfg(not(feature = "e2e"))]
pub(crate) fn options() -> WgpuConfiguration {
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    if let Some(one) = only(wgpu::Backends::from_env(), a_gpu_answers) {
        setup.instance_descriptor.backends = one;
    }
    let device = std::sync::Arc::clone(&setup.device_descriptor);
    setup.device_descriptor = std::sync::Arc::new(move |adapter| sparing(device(adapter)));
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

/// The one API to open, or `None` for eframe's choice: `WGPU_BACKEND` when it is set, and every
/// API when no GPU answers the system's own — OpenGL is then tried before the software adapter.
#[cfg(not(feature = "e2e"))]
fn only(
    asked: Option<wgpu::Backends>,
    a_gpu_answers: impl FnOnce(wgpu::Backends) -> bool,
) -> Option<wgpu::Backends> {
    let own = cfg!(windows).then_some(wgpu::Backends::DX12)?;
    (asked.is_none() && a_gpu_answers(own)).then_some(own)
}

/// Asked of an instance of its own, dropped at once: eframe's opens with what this answers.
#[cfg(not(feature = "e2e"))]
fn a_gpu_answers(backends: wgpu::Backends) -> bool {
    let mut descriptor = WgpuSetupCreateNew::without_display_handle().instance_descriptor;
    descriptor.backends = backends;
    let instance = wgpu::Instance::new(descriptor);
    ready(instance.enumerate_adapters(backends)).is_some_and(|adapters| {
        adapters
            .iter()
            .any(|adapter| adapter.get_info().device_type != wgpu::DeviceType::Cpu)
    })
}

/// wgpu's native futures are ready when asked: polled once, never waited for.
#[cfg(not(feature = "e2e"))]
fn ready<T>(future: impl Future<Output = T>) -> Option<T> {
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match std::pin::pin!(future).poll(&mut context) {
        std::task::Poll::Ready(answer) => Some(answer),
        std::task::Poll::Pending => None,
    }
}

/// wgpu's default reserves 128 MB of video memory and 64 MB of RAM a block; a window of text
/// fills neither.
#[cfg(not(feature = "e2e"))]
fn sparing(mut device: wgpu::DeviceDescriptor<'static>) -> wgpu::DeviceDescriptor<'static> {
    device.memory_hints = wgpu::MemoryHints::MemoryUsage;
    device
}

#[cfg(all(test, not(feature = "e2e")))]
mod tests {
    use super::*;

    #[test]
    fn the_system_s_own_api_is_opened_alone_when_a_gpu_answers_it() {
        let expected = cfg!(windows).then_some(wgpu::Backends::DX12);

        assert_eq!(only(None, |_| true), expected);
    }

    #[test]
    fn without_a_gpu_or_with_an_api_asked_for_the_choice_stays_eframe_s() {
        assert_eq!(only(None, |_| false), None);
        assert_eq!(only(Some(wgpu::Backends::VULKAN), |_| true), None);
    }

    #[test]
    fn the_device_is_asked_to_reserve_memory_by_small_blocks() {
        let device = sparing(wgpu::DeviceDescriptor::default());

        assert!(matches!(
            device.memory_hints,
            wgpu::MemoryHints::MemoryUsage
        ));
    }

    #[test]
    fn a_future_that_is_not_ready_is_not_waited_for() {
        assert_eq!(ready(std::future::ready(7)), Some(7));
        assert_eq!(ready(std::future::pending::<u8>()), None);
    }
}
