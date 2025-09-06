// src/lib/viewer.rs
use anyhow::Result;
// use bytemuck::{Pod, Zeroable};
use egui_wgpu::wgpu;
use glam::{Quat, Vec3}; //Mat4, Vec4
// use std::io::Read;
use std::path::Path;
use std::sync::Arc; //mpsc, Mutex
use wgpu::util::DeviceExt;
// use wgpu::{
//     BlendState, ColorTargetState, ColorWrites, Device, FragmentState, PrimitiveState, Queue,
//     RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor,
//     SurfaceConfiguration, TextureFormat, VertexBufferLayout, VertexState, VertexStepMode,
// };
//

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[derive(Clone)]
pub struct GltfViewer {
    pub rotation: Quat,
    pub vertex_buffer: Option<Arc<wgpu::Buffer>>,
    pub index_buffer: Option<Arc<wgpu::Buffer>>,
    pub index_count: u32,
    pub render_pipeline: Option<Arc<wgpu::RenderPipeline>>,
}

impl GltfViewer {
    const SHADER: &str = r#"
        @vertex
        fn vs_main(@location(0) in_position: vec3<f32>) -> @builtin(position) vec4<f32> {
            return vec4<f32>(in_position, 1.0);
        }

        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let vertices = vec![
            Vertex {
                position: [-0.5, -0.5, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
            },
            Vertex {
                position: [0.0, 0.5, 0.0],
            },
        ];
        let indices: Vec<u16> = vec![0, 1, 2];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::SHADER.into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Nexusview Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            rotation: Quat::IDENTITY,
            vertex_buffer: Some(Arc::new(vertex_buffer)),
            index_buffer: Some(Arc::new(index_buffer)),
            index_count: indices.len() as u32,
            render_pipeline: Some(Arc::new(render_pipeline)),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        todo!();
    }

    pub fn dummy() -> Self {
        Self {
            rotation: Quat::IDENTITY,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            render_pipeline: None,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        let angle = delta_time * 0.8;
        self.rotation = Quat::from_axis_angle(Vec3::Y, angle) * self.rotation;
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let (Some(vertex_buffer), Some(index_buffer), Some(render_pipeline)) = (
            &self.vertex_buffer.as_ref(),
            &self.index_buffer.as_ref(),
            &self.render_pipeline.as_ref(),
        ) {
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }

    pub fn load_gltf<P: AsRef<Path>>(&mut self, _path: P) -> Result<(), String> {
        // TODO: Implement proper GLTF loading to update vertices and indices  and rebuild the buffers.
        Ok(())
    }
}
