//! Drawing below the screen's resolution: the scene renders into an
//! offscreen image at the chosen size (1080p on a 4K TV, say) and one pass
//! stretches it over the screen. A Pi 4 can't fill 4K with the LED effects
//! at a useful frame rate, but it can scale a 1080p image up for almost
//! nothing.

/// Stretches the offscreen image onto the screen with linear filtering.
const SHADER: &str = r"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var blur_sampler: sampler;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    // One triangle that covers the screen.
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var o: Out;
    o.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2<f32>(x, y);
    return o;
}

@fragment
fn fs(v: Out) -> @location(0) vec4<f32> {
    return textureSample(image, blur_sampler, v.uv);
}
";

/// The offscreen image the scene draws into, and the pass that shows it.
#[derive(Debug)]
pub struct Upscaler {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
    target: Option<Target>,
}

#[derive(Debug)]
struct Target {
    size: (u32, u32),
    view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

impl Upscaler {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Upscaler {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("upscale"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("upscale"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("upscale"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("upscale"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("upscale"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Upscaler { pipeline, layout, sampler, format, target: None }
    }

    /// The image to draw the scene into at `size`, made (or remade) as
    /// needed.
    pub fn target(&mut self, device: &wgpu::Device, size: (u32, u32)) -> &wgpu::TextureView {
        let target = match self.target.take() {
            Some(t) if t.size == size => t,
            _ => self.make(device, size),
        };
        &self.target.insert(target).view
    }

    fn make(&self, device: &wgpu::Device, size: (u32, u32)) -> Target {
        {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scene at render size"),
                size: wgpu::Extent3d { width: size.0.max(1), height: size.1.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("upscale"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            Target { size, view, bind }
        }
    }

    /// Stretches the last image drawn over `screen`.
    pub fn draw(&self, device: &wgpu::Device, queue: &wgpu::Queue, screen: &wgpu::TextureView) {
        let Some(t) = &self.target else { return };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("upscale") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("upscale"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &t.bind, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

/// The size to draw at for a `screen` of this size: the screen's own when
/// it's no taller than `max_height`, else scaled down to that height
/// (keeping the shape).
pub fn render_size(screen: (u32, u32), max_height: Option<u32>) -> (u32, u32) {
    match max_height {
        Some(h) if screen.1 > h && h > 0 => {
            let w = (u64::from(screen.0) * u64::from(h) / u64::from(screen.1.max(1))) as u32;
            (w.max(1), h)
        }
        _ => screen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_4k_screen_draws_at_1080p_and_smaller_screens_as_they_are() {
        assert_eq!(render_size((3840, 2160), Some(1080)), (1920, 1080));
        assert_eq!(render_size((1920, 1080), Some(1080)), (1920, 1080));
        assert_eq!(render_size((1366, 768), Some(1080)), (1366, 768), "never scaled up");
        assert_eq!(render_size((3840, 2160), None), (3840, 2160), "native");
        assert_eq!(render_size((3840, 2160), Some(720)), (1280, 720));
    }
}
