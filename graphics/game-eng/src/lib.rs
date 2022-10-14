mod framework;
mod texture;
use texture::Texture;

use bytemuck::{Pod, Zeroable};
use std::num::{NonZeroU32, NonZeroU64};
use wgpu::util::DeviceExt;

use std::io;
use std::fs::File;
use std::io::Read;


#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 2],
    _tex_coord: [f32; 2],
    _index: u32,
}

macro_rules! vertex {
    ($pos:expr,$tex_coord:expr,$index:expr) => {
        Vertex {
            _pos: $pos,
            _tex_coord: $tex_coord,
            _index: $index
        }
    };
}

fn create_vertices() -> Vec<Vertex> {
    vec![
        // left rectangle
        vertex!([-1.0, -1.0], [0.0, 1.0], 0),
        vertex!([-1.0, 1.0], [0.0, 0.0], 0),
        vertex!([0.0, 1.0], [1.0, 0.0], 0),
        vertex!([0.0, -1.0], [1.0, 1.0], 0),
        // right rectangle
        vertex!([0.0, -1.0], [0.0, 1.0], 1),
        vertex!([0.0, 1.0], [0.0, 0.0], 1),
        vertex!([1.0, 1.0], [1.0, 0.0], 1),
        vertex!([1.0, -1.0], [1.0, 1.0], 1),
    ]
}

fn create_indices() -> Vec<u16> {
vec![
        // Left rectangle
        0, 1, 2, // 1st
        2, 0, 3, // 2nd
        // Right rectangle
        4, 5, 6, // 1st
        6, 4, 7, // 2nd
    ]
}

struct Arguments {
    image_array: Vec<Texture>
}
type A = Arguments;

struct Tex {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    index_count: u32,
}

impl framework::Base for Tex {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::TEXTURE_BINDING_ARRAY
    }

    type A = Arguments;
    fn init(
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        arguments: A,
    ) -> Self {
        let mut uniform_workaround = false;
        let base_shader_module = device.create_shader_module(wgpu::include_wgsl!("indexing.wgsl"));
        let env_override = match std::env::var("WGPU_TEXTURE_ARRAY_STYLE") {
            Ok(value) => match &*value.to_lowercase() {
                "nonuniform" | "non_uniform" => Some(true),
                "uniform" => Some(false),
                _ => None,
            },
            Err(_) => None,
        };
        let fragment_entry_point = match (device.features(), env_override) {
            (_, Some(true)) => "non_uniform_main",
            (f, _)
                if f.contains(
                    wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                ) =>
            {
                "non_uniform_main"
            }
            _ => {
                panic!("your device does not support the required features");
            }
        };

        // let fragment_entry_point = "uniform_main";
        // let uniform_workaround = true;
        // TODO: Because naga's capibilities are evaluated on validate, not on write, we cannot make a shader module with unsupported
        // capabilities even if we don't use it. So for now put it in a separate module.
        let fragment_shader_module = device.create_shader_module(wgpu::include_wgsl!("non_uniform_indexing.wgsl"));
        log::info!("Using fragment entry point '{}'", fragment_entry_point);

        let vertex_size = std::mem::size_of::<Vertex>();
        let vertex_data = create_vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_data = create_indices();
        let index_count = index_data.len() as u32;
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        let image_tex = Texture::from_bytes(&device, &queue, include_bytes!("C:/Users/Soham's laptop/Documents/2d-info-projekt/assets/test.png"), "test ai horse image").unwrap();
        let image_tex2 = Texture::from_bytes(&device, &queue, include_bytes!("C:/Users/Soham's laptop/Documents/2d-info-projekt/assets/test2.jpg"), "test cypberpunk art").unwrap();
        let image_array = arguments.image_array.iter().map(|path| {
            let bytes = std::fs::read(path).unwrap();
        });
        let image_tex_array: Vec<_> = image_array.iter().map(|f| &(f.view)).collect();
        let image_sampler_array: Vec<_> = image_array.iter().map(|f| &(f.sampler)).collect();
        println!("{}",include_bytes!("C:/Users/Soham's laptop/Documents/2d-info-projekt/assets/test.png").len());


        let texture_index_buffer_contents = vec![0; 256*image_array.len()];
        let texture_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&texture_index_buffer_contents),
            usage: wgpu::BufferUsages::UNIFORM,
        });




        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(image_array.len() as u32),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: NonZeroU32::new(image_array.len() as u32),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(NonZeroU64::new(4).unwrap()),
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&image_tex_array[..]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::SamplerArray(&image_sampler_array[..]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &texture_index_buffer,
                        offset: 0,
                        size: Some(NonZeroU64::new(4).unwrap()),
                    }),
                },
            ],
            layout: &bind_group_layout,
            label: Some("bind group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let index_format = wgpu::IndexFormat::Uint16;

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &base_shader_module,
                entry_point: "vert_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: vertex_size as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Sint32],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader_module,
                entry_point: fragment_entry_point,
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
            index_format,
            index_count
        }
    }
    fn resize(
        &mut self,
        _sc_desc: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        // noop
    }
    fn update(&mut self, _event: winit::event::WindowEvent) {
        // noop
    }
    fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _spawner: &framework::Spawner,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("primary"),
        });

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), self.index_format);

            rpass.set_bind_group(0, &self.bind_group, &[0]);
            rpass.draw_indexed(0..self.index_count, 0, 0..1);

        drop(rpass);

        queue.submit(Some(encoder.finish()));
    }
}

pub fn run() {
    framework::run::<Tex>("texture-arrays", Arguments { image_array: [] });
}