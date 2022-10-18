use crate::base::{Base, Window, BuilderOrderChecker};
use bytemuck::{Pod, Zeroable};
use std::{num::{NonZeroU32, NonZeroU64}, sync::{Arc, RwLock}};
use wgpu::util::DeviceExt;
use crate::texture::Texture as ImageTexture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub tex_coord: [f32; 2],
    pub index: u32,
}

macro_rules! vertex {
    ($pos:expr,$tex_coord:expr,$index:expr) => {
        {use crate::tex_array::Vertex;
        Vertex {
            pos: $pos,
            tex_coord: $tex_coord,
            index: $index
        }}
    };
}

pub(crate) use vertex;



// REQUIRED_FEATURES: wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING | wgpu::Features::TEXTURE_BINDING_ARRAY
#[allow(dead_code, non_snake_case)]
pub fn REQUIRED_FEATURES() -> wgpu::Features {
    wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING | wgpu::Features::TEXTURE_BINDING_ARRAY
}

pub struct TextureRenderer {
    // requiered external components
    base: Arc<RwLock<Base>>,
    window: Arc<RwLock<Window>>,

    // inner contents
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    index_count: u32,
    
}


pub struct TextureRendererBuilder {
    // requiered external components
    base: Arc<RwLock<Base>>,
    window: Arc<RwLock<Window>>,

    // inner contents
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group: Option<wgpu::BindGroup>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    index_format: Option<wgpu::IndexFormat>,

    // helper variables
    bind_group_layout: Option<wgpu::BindGroupLayout>,

    // order checking
    stage: BuilderOrderChecker<TextureRendererBuildOrder>,
}

const VERTEX_SIZE: usize = std::mem::size_of::<Vertex>();

impl TextureRenderer {
    pub fn init(base: Arc<RwLock<Base>>, window: Arc<RwLock<Window>>) -> TextureRendererBuilder{
        TextureRendererBuilder { base, window, pipeline: None, bind_group: None, vertex_buffer: None, index_buffer: None, index_format: None, bind_group_layout: None, stage: BuilderOrderChecker::new(TextureRendererBuildOrder::BaseAndWindow) }
    }  

    // currently very inefficient
    // TODO:
    pub fn modify_vertex_buffer(&mut self, vertices: Vec<Vertex>) {
        let vertex_buffer = (*self.base.read().unwrap()).device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.vertex_buffer =  vertex_buffer;
       
    }

    // currently very inefficient
    // TODO:
    pub fn modify_index_buffer(&mut self, indices: Vec<u16>) {
        let index_buffer = (*self.base.read().unwrap()).device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.index_count = indices.len() as u32;
        drop(std::mem::replace(&mut self.index_buffer, index_buffer));
    }
    

    pub fn render(&self, view: &wgpu::TextureView) {
        let mut encoder = (*self.base.read().unwrap()).device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

        (*self.base.read().unwrap()).queue.submit(Some(encoder.finish()));
    }
} 

#[derive(PartialEq, PartialOrd, Debug)]
enum TextureRendererBuildOrder {
    BaseAndWindow,
    CreatePipeline,
    CreateBindGroup,
}

impl TextureRendererBuilder {
    pub fn create_vertex_buffer(mut self) -> Self {
        let vertex_buffer = (*self.base.read().unwrap()).device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice::<Vertex,_>(&[]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.vertex_buffer = Some(vertex_buffer);
        self
    }

    pub fn create_index_buffer(mut self) -> Self {
        let index_buffer = (*self.base.read().unwrap()).device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice::<u8,_>(&[]),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.index_buffer = Some(index_buffer);
        self
    }

    pub fn create_pipeline(mut self, max_texture_count: u32) -> Self {

        self.stage.check_and_change(TextureRendererBuildOrder::CreatePipeline);

        let shader_module = (*self.base.read().unwrap()).device.create_shader_module(wgpu::include_wgsl!("non_uniform_indexing.wgsl"));
        // let base_shader_module = self.base.device.create_shader_module(wgpu::include_wgsl!("indexing.wgsl"));

        let bind_group_layout = (*self.base.read().unwrap()).device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    count: NonZeroU32::new(max_texture_count),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: NonZeroU32::new(max_texture_count),
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

        let pipeline_layout = (*self.base.read().unwrap()).device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = (*self.base.read().unwrap()).device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vert_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: VERTEX_SIZE as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Sint32],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "non_uniform_main",
                targets: &[Some((*self.base.read().unwrap()).config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        self.pipeline = Some(pipeline);
        self.bind_group_layout = Some(bind_group_layout);
        self
        
    }

    pub fn set_index_format(mut self, index_format: wgpu::IndexFormat) -> Self{
        self.index_format = Some(index_format);
        self
    }

    /// makes textures accesible to gpu 
    pub fn create_bind_group(mut self, textures: Vec<&'static ImageTexture>) -> Self{
        self.stage.check_and_change(TextureRendererBuildOrder::CreateBindGroup);

        let bind_group = (*self.base.read().unwrap()).device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray({
                        &textures.iter().map(|tex|{
                            &tex.view
                        }).collect::<Vec<_>>()[..]
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::SamplerArray({
                        &textures.iter().map(|tex|{
                            &tex.sampler
                        }).collect::<Vec<_>>()[..]
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &(*self.base.read().unwrap()).device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Padding Buffer"),
                            contents: bytemuck::cast_slice::<u8,_>(&[0]),
                            usage: wgpu::BufferUsages::UNIFORM,
                            }),
                        offset: 0,
                        size: Some(NonZeroU64::new(4).unwrap()),
                    }),
                },
            ],
            layout: self.bind_group_layout.as_ref().unwrap(),
            label: Some("bind group"),
        });

        self.bind_group = Some(bind_group);
        self

    }

    pub fn build(self) -> TextureRenderer{
        TextureRenderer { base: self.base, window: self.window, pipeline: self.pipeline.expect("pipeline not set"), bind_group: self.bind_group.expect("bind_group not set"), vertex_buffer: self.vertex_buffer.expect("vertex_buffer not set"), index_buffer: self.index_buffer.expect("index_buffer not set"), index_format: wgpu::IndexFormat::Uint16, index_count: 0 }
    }

}

