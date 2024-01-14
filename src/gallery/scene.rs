use async_std::sync::RwLock;
use async_trait::async_trait;
use winit::event_loop::EventLoopProxy;

use crate::{
    core::{app::App, command_queue::Command, events::CommandEvent},
    renderer::sun::Viewport,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct Scene {
    pub world: Arc<RwLock<bevy_ecs::world::World>>,
    pub pipelines: Arc<RwLock<HashMap<String, wgpu::RenderPipeline>>>,
    pub commands: Vec<Command>,
}

impl Scene {
    pub fn new() -> Self {
        Scene::default()
    }

    pub async fn create_pipeline(
        &mut self,
        name: String,
        device: Arc<wgpu::Device>,
        viewport: Arc<RwLock<Viewport>>,
        shader_src: impl AsRef<str>,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&name),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(shader_src.as_ref())),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&name),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&name),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: viewport.read().await.get_config().format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
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

        self.pipelines.write().await.insert(name, pipeline);
    }
}

#[async_trait(?Send)]
impl App for Scene {
    fn init(&mut self, _init_commands: Vec<crate::prelude::command_queue::Command>) {}

    fn process_command(&mut self, _cmd: Command) {}

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    async fn process_event(
        &mut self,
        event: &winit::event::Event<crate::core::events::CommandEvent>,
        elp: EventLoopProxy<CommandEvent>,
    ) {
        match event {
            winit::event::Event::WindowEvent {
                window_id,
                event: winit::event::WindowEvent::RedrawRequested,
            } => {
                use crate::core::events::RenderDesc;

                let render_desc = RenderDesc {
                    world: self.world.clone(),
                    pipelines: self.pipelines.clone(),
                    window_id: *window_id,
                };
                elp.send_event(CommandEvent::Render(render_desc)).unwrap();
            }
            winit::event::Event::UserEvent(CommandEvent::RequestPipeline(render_desc)) => {
                let render_desc = render_desc.clone();
                self.create_pipeline(
                    render_desc.name,
                    render_desc.device,
                    render_desc.viewport,
                    render_desc.shader_src,
                )
                .await;
            }
            _ => {}
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
