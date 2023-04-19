pub mod transformation;

use transformation::Transformation;

use bytemuck::{Pod, Zeroable};
use std::mem;
use wgpu::util::DeviceExt;

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Quad {
    /// The position of the [`Quad`].
    pub position: [f32; 2],

    /// The size of the [`Quad`].
    pub size: [f32; 2],

    /// The color of the [`Quad`], in __linear RGB__.
    pub color: [f32; 4],

    /// The border color of the [`Quad`], in __linear RGB__.
    pub border_color: [f32; 4],

    /// The border radius of the [`Quad`].
    pub border_radius: [f32; 4],

    /// The border width of the [`Quad`].
    pub border_width: f32,
}

#[allow(unsafe_code)]
unsafe impl bytemuck::Zeroable for Quad {}

#[allow(unsafe_code)]
unsafe impl bytemuck::Pod for Quad {}

/// A rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rectangle<T = f32> {
    /// X coordinate of the top-left corner.
    pub x: T,

    /// Y coordinate of the top-left corner.
    pub y: T,

    /// Width of the rectangle.
    pub width: T,

    /// Height of the rectangle.
    pub height: T,
}

#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::BindGroup,
    uniforms_buffer: wgpu::Buffer,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    instances: wgpu::Buffer,
}

impl Pipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Pipeline {
        let constant_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("quad uniforms layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            mem::size_of::<Uniforms>() as wgpu::BufferAddress,
                        ),
                    },
                    count: None,
                }],
            });

        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad uniforms buffer"),
            size: mem::size_of::<Uniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniforms = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad uniforms bind group"),
            layout: &constant_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad pipeline layout"),
            push_constant_ranges: &[],
            bind_group_layouts: &[&constant_layout],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "quad.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: mem::size_of::<Quad>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array!(
                            1 => Float32x2,
                            2 => Float32x2,
                            3 => Float32x4,
                            4 => Float32x4,
                            5 => Float32x4,
                            6 => Float32,
                        ),
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad vertex buffer"),
            contents: bytemuck::cast_slice(&QUAD_VERTS),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index buffer"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad instance buffer"),
            size: mem::size_of::<Quad>() as u64 * MAX_INSTANCES as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Pipeline {
            pipeline,
            uniforms,
            uniforms_buffer,
            vertices,
            indices,
            instances,
        }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        instances: &[Quad],
        transformation: Transformation,
        scale: f32,
        bounds: Rectangle<u32>,
    ) {
        let a = [0.001510574, 0.0, 0.0, 0.0, 0.0, -0.002283105, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0];
        let uniforms = Uniforms::from_a(a, scale);

        println!("uniforms {:?}", uniforms);
        {
            let mut constants_buffer = staging_belt.write_buffer(
                encoder,
                &self.uniforms_buffer,
                0,
                wgpu::BufferSize::new(mem::size_of::<Uniforms>() as u64).unwrap(),
                device,
            );

            constants_buffer.copy_from_slice(bytemuck::bytes_of(&uniforms));
        }

        let mut i = 0;
        let total = instances.len();

        while i < total {
            let end = (i + MAX_INSTANCES).min(total);
            let amount = end - i;

            let instance_bytes = bytemuck::cast_slice(&instances[i..end]);

            let mut instance_buffer = staging_belt.write_buffer(
                encoder,
                &self.instances,
                0,
                wgpu::BufferSize::new(instance_bytes.len() as u64).unwrap(),
                device,
            );

            instance_buffer.copy_from_slice(instance_bytes);

            let mut render_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("quad render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniforms, &[]);
            render_pass
                .set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, self.vertices.slice(..));
            render_pass.set_vertex_buffer(1, self.instances.slice(..));

            // render_pass.set_scissor_rect(
            //     bounds.x,
            //     bounds.y,
            //     bounds.width,
            //     // TODO: Address anti-aliasing adjustments properly
            //     bounds.height,
            // );

            render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..amount as u32);

            i += MAX_INSTANCES;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct Vertex {
    _position: [f32; 2],
}

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

const QUAD_VERTS: [Vertex; 4] = [
    Vertex {
        _position: [0.0, 0.0],
    },
    Vertex {
        _position: [1.0, 0.0],
    },
    Vertex {
        _position: [1.0, 1.0],
    },
    Vertex {
        _position: [0.0, 1.0],
    },
];

const MAX_INSTANCES: usize = 100_000;

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct Uniforms {
    transform: [f32; 16],
    scale: f32,
    // Uniforms must be aligned to their largest member,
    // this uses a mat4x4<f32> which aligns to 16, so align to that
    _padding: [f32; 3],
}

impl Uniforms {
    fn new(transformation: Transformation, scale: f32) -> Uniforms {
        Self {
            transform: *transformation.as_ref(),
            scale,
            _padding: [0.0; 3],
        }
    }

    fn from_a(transformation: [f32;16], scale: f32) -> Uniforms {
        Self {
            transform: transformation,
            scale,
            _padding: [0.0; 3],
        }
    }
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            transform: *Transformation::identity().as_ref(),
            scale: 1.0,
            _padding: [0.0; 3],
        }
    }
}
