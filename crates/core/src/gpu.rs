use std::sync::Arc;

use tracing::info;
use wgpu::{
    Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, PowerPreference, Queue,
    RequestAdapterOptions, Surface, SurfaceConfiguration, TextureUsages,
};
use winit::window::Window;

use crate::error::MyceliumError;

/// owns the wgpu instance, adapter, device, queue, and surface.
pub struct GpuContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
}

impl GpuContext {
    pub async fn new(window: Arc<Window>) -> Result<Self, MyceliumError> {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|e| MyceliumError::GpuInit(e.to_string()))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| MyceliumError::GpuInit(e.to_string()))?;

        info!("GPU adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("mycelium"),
                ..Default::default()
            })
            .await
            .map_err(|e| MyceliumError::GpuInit(e.to_string()))?;

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
        })
    }

    /// resize the surface. call when window size changes.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }
}
