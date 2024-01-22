use std::collections::HashMap;

use wgpu::Device;

use super::sun::{ResourceID, Viewport};

pub struct SunPipeline {
    pub id: uuid::Uuid,

    pub window: winit::window::WindowId,

    pub bind_group_layouts: Vec<wgpu::BindGroupLayout>,
    pub bind_groups: HashMap<ResourceID, (u32, wgpu::BindGroup)>,

    pub pipeline: wgpu::RenderPipeline,
}

impl SunPipeline {
    pub fn new(
        device: &Device,
        viewport: &Viewport,
        name: String,
        shader_src: impl AsRef<str>,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'static>],
        bind_group_layout_descs: Vec<wgpu::BindGroupLayoutDescriptor<'static>>,
        topology: wgpu::PrimitiveTopology,
    ) -> Self {
        let mut bind_group_layouts = Vec::new();

        for index in 0..bind_group_layout_descs.len() as u32 {
            let bind_group_layout =
                device.create_bind_group_layout(&bind_group_layout_descs[index as usize]);

            bind_group_layouts.push(bind_group_layout);
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&name),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(shader_src.as_ref())),
        });

        let layouts = bind_group_layouts.iter().collect::<Vec<&_>>();

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&name),
            bind_group_layouts: layouts.as_slice(),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&name),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: vertex_buffer_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: viewport.get_config().format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            id: uuid::Uuid::new_v4(),
            window: viewport.desc.window.id(),
            bind_group_layouts,
            bind_groups: HashMap::new(),
            pipeline,
        }
    }

    pub fn add_bind_group(
        &mut self,
        device: &Device,
        resource_id: uuid::Uuid,
        label: &str,
        layout_index: usize,
        entries: &[wgpu::BindGroupEntry],
    ) {
        let bg_layout = &self.bind_group_layouts[layout_index];

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: bg_layout,
            entries,
        });

        self.bind_groups
            .insert(resource_id, (layout_index as u32, bind_group));
    }
}
