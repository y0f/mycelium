use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    ColorTargetState, Device, FragmentState, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderStages, TextureFormat, VertexState,
};

/// uniform data pushed to the GPU every frame. all f32 fields, no arrays/vecs
/// to avoid WGSL uniform alignment issues. total: 32 f32 = 128 bytes.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    /// elapsed time in seconds.
    pub time: f32,
    /// viewport width in pixels.
    pub res_x: f32,
    /// viewport height in pixels.
    pub res_y: f32,
    /// beat phase, 0.0 at beat, 1.0 just before next.
    pub beat_phase: f32,
    pub band0: f32,
    pub band1: f32,
    pub band2: f32,
    pub band3: f32,
    pub band4: f32,
    pub band5: f32,
    pub bpm: f32,
    /// onset detection flag, 1.0 on onset else 0.0.
    pub onset: f32,
    pub sband0: f32,
    pub sband1: f32,
    pub sband2: f32,
    pub sband3: f32,
    pub sband4: f32,
    pub sband5: f32,
    /// overall RMS energy.
    pub energy: f32,
    /// spectral centroid (brightness) in Hz.
    pub spectral_centroid: f32,
    pub speed: f32,
    pub intensity: f32,
    pub zoom: f32,
    pub color_shift: f32,
    pub rotation_speed: f32,
    pub bass_reactivity: f32,
    pub flash_intensity: f32,
    pub brightness: f32,
    pub pband0: f32,
    pub pband1: f32,
    pub pband2: f32,
    pub pband3: f32,
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            time: 0.0,
            res_x: 1920.0,
            res_y: 1080.0,
            beat_phase: 0.0,
            band0: 0.0, band1: 0.0, band2: 0.0, band3: 0.0, band4: 0.0, band5: 0.0,
            bpm: 0.0,
            onset: 0.0,
            sband0: 0.0, sband1: 0.0, sband2: 0.0, sband3: 0.0, sband4: 0.0, sband5: 0.0,
            energy: 0.0,
            spectral_centroid: 0.0,
            speed: 1.0,
            intensity: 1.0,
            zoom: 1.0,
            color_shift: 0.0,
            rotation_speed: 1.0,
            bass_reactivity: 2.0,
            flash_intensity: 0.2,
            brightness: 1.0,
            pband0: 0.0, pband1: 0.0, pband2: 0.0, pband3: 0.0,
        }
    }
}

/// draws a fragment shader over the entire screen.
pub struct FullscreenQuad {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    uniform_buffer: Buffer,
}

impl FullscreenQuad {
    pub fn new(device: &Device, format: TextureFormat, shader_source: &str) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("fullscreen_quad"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("quad_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("quad_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("quad_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("quad_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("quad_bind_group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            bind_group,
            uniform_buffer,
        }
    }

    /// update uniform data on the GPU.
    pub fn update_uniforms(&self, queue: &Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }
}
