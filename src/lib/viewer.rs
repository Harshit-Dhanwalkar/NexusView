// src/lib/viewer.rs
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use egui_wgpu::wgpu;
use glam::{Mat4, Quat, Vec3};
use std::io::Read;
use std::path::Path;
use wgpu::util::DeviceExt;
use wgpu::{
    BlendState, ColorTargetState, ColorWrites, Device, FragmentState, PrimitiveState, Queue,
    RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor,
    SurfaceConfiguration, TextureFormat, VertexBufferLayout, VertexState, VertexStepMode,
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    fn desc() -> VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

pub struct GltfViewer {
    pub render_pipeline: RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub view_proj_matrix: Mat4,
    pub rotation: Quat,
}

impl GltfViewer {
    const SHADER: &str = r#"
        // Simple 3D shader
        @vertex
        fn vs_main(@location(0) in_position: vec3<f32>) -> @builtin(position) vec4<f32> {
            return vec4<f32>(in_position, 1.0);
        }

        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        // Hardcoded triangle for placeholder
        let vertices = [
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

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::SHADER.into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            view_proj_matrix: Mat4::IDENTITY,
            rotation: Quat::IDENTITY,
        }
    }

    pub fn load_gltf(&mut self, path: &Path) -> Result<()> {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let glb = gltf::Glb::from_slice(&buffer)?;
        // let (_gltf, buffers, _images) = gltf::import_slice(&glb.json, &glb.bin)?;
        let (_gltf, buffers, _images) = gltf::import_slice(&glb.json)?;

        println!(
            "Loaded GLB file: {} (placeholder implementation)",
            path.display()
        );

        // TODO: Implement proper GLTF parsing and buffer creation

        Ok(())
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    pub fn update(&mut self, _delta_time: f32) {
        // Implement rotation logic later
        // self.rotation = Quat::from_axis_angle(Vec3::Y, _delta_time) * self.rotation;
    }
}
