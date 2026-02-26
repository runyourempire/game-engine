use std::path::Path;

use image::{ImageBuffer, Rgba};
use wgpu::util::DeviceExt;

use crate::codegen::CompileOutput;

/// Headless GPU renderer for visual snapshot testing.
pub struct SnapshotRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl SnapshotRenderer {
    /// Create a new renderer using the best available GPU.
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .ok_or("no GPU adapter found")?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("snapshot"),
                ..Default::default()
            },
            None,
        ))
        .map_err(|e| format!("device request failed: {e}"))?;

        Ok(Self { device, queue })
    }

    /// Render a single frame and return RGBA pixel data.
    pub fn render_frame(
        &self,
        output: &CompileOutput,
        width: u32,
        height: u32,
        time: f32,
    ) -> Result<Vec<u8>, String> {
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        // Create render target
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("snapshot-target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Build uniform data: [resolution.x, resolution.y, time, frame, ...params]
        let mut uniform_floats: Vec<f32> = vec![
            width as f32,
            height as f32,
            time,
            0.0, // frame
        ];
        for p in &output.params {
            uniform_floats.push(p.base_value as f32);
        }
        // Pad to match uniform_float_count (accounts for audio-derived params etc)
        while uniform_floats.len() < output.uniform_float_count {
            uniform_floats.push(0.0);
        }
        let uniform_bytes = floats_to_bytes(&uniform_floats);

        // Uniform buffer
        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: &uniform_bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Compile shader
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("game-shader"),
                source: wgpu::ShaderSource::Wgsl(output.wgsl.as_str().into()),
            });

        // Create pipeline (auto layout derives from shader)
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("game-pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Bind group — binding 0 is always uniforms
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let mut entries: Vec<wgpu::BindGroupEntry> = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }];

        // If shader uses audio, add binding 1 with zeros
        let audio_buffer;
        if output.uses_audio {
            audio_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("audio"),
                contents: &[0u8; 16], // 4 f32 zeros
                usage: wgpu::BufferUsages::UNIFORM,
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: audio_buffer.as_entire_binding(),
            });
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game-bind"),
            layout: &bind_group_layout,
            entries: &entries,
        });

        // Readback buffer (aligned to 256 bytes per row)
        let bytes_per_row = width * 4;
        let padded_bytes_per_row = ((bytes_per_row + 255) / 256) * 256;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded_bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Render
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("snapshot-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..4, 0..1);
        }

        // Copy texture to readback buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );

        self.queue.submit(Some(encoder.finish()));

        // Map and read
        let slice = readback_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|e| format!("map recv error: {e}"))?
            .map_err(|e| format!("map error: {e}"))?;

        // Extract pixel data (remove row padding)
        let data = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        readback_buffer.unmap();

        Ok(pixels)
    }
}

/// Save RGBA pixel data as a PNG file.
pub fn save_png(pixels: &[u8], width: u32, height: u32, path: &Path) -> Result<(), String> {
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(width, height, pixels.to_vec())
            .ok_or("invalid pixel dimensions")?;
    img.save(path).map_err(|e| format!("PNG save error: {e}"))
}

/// Load a PNG file and return RGBA pixel data.
pub fn load_png(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let img = image::open(path).map_err(|e| format!("PNG load error: {e}"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((rgba.into_raw(), w, h))
}

/// Compare two RGBA pixel buffers. Returns similarity percentage (0-100).
/// Pixels with per-channel difference <= threshold are considered matching.
pub fn compare_pixels(actual: &[u8], reference: &[u8], channel_threshold: i32) -> f64 {
    if actual.len() != reference.len() || actual.is_empty() {
        return 0.0;
    }
    let total_pixels = actual.len() / 4;
    let mut matching = 0usize;
    for i in 0..total_pixels {
        let base = i * 4;
        let dr = (actual[base] as i32 - reference[base] as i32).abs();
        let dg = (actual[base + 1] as i32 - reference[base + 1] as i32).abs();
        let db = (actual[base + 2] as i32 - reference[base + 2] as i32).abs();
        if dr <= channel_threshold && dg <= channel_threshold && db <= channel_threshold {
            matching += 1;
        }
    }
    (matching as f64 / total_pixels as f64) * 100.0
}

/// Generate a visual diff image highlighting differing pixels in red.
pub fn generate_diff(actual: &[u8], reference: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut diff = Vec::with_capacity(actual.len());
    let total_pixels = actual.len() / 4;
    for i in 0..total_pixels {
        let base = i * 4;
        let dr = (actual[base] as i32 - reference[base] as i32).abs();
        let dg = (actual[base + 1] as i32 - reference[base + 1] as i32).abs();
        let db = (actual[base + 2] as i32 - reference[base + 2] as i32).abs();
        if dr > 2 || dg > 2 || db > 2 {
            diff.extend_from_slice(&[255, 0, 0, 255]); // red = different
        } else {
            // Dim the matching pixels
            diff.push(actual[base] / 3);
            diff.push(actual[base + 1] / 3);
            diff.push(actual[base + 2] / 3);
            diff.push(255);
        }
    }
    let _ = (width, height); // used for context
    diff
}

fn floats_to_bytes(floats: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(floats.len() * 4);
    for f in floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    // Pad to 16-byte alignment (wgpu uniform buffer requirement)
    while bytes.len() % 16 != 0 {
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
    }
    bytes
}
