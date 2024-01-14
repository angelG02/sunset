use std::{collections::HashMap, sync::Arc};

use async_std::sync::RwLock;
use async_trait::async_trait;
use tracing::info;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopProxy,
    window::Window,
};

use crate::{
    core::{
        app::App,
        command_queue::Command,
        events::{CommandEvent, PipelineDesc},
    },
    prelude::{events::RenderDesc, Asset, AssetType},
};

pub struct Sun {
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    device: Option<Arc<wgpu::Device>>,
    queue: Option<wgpu::Queue>,

    pub viewports: HashMap<winit::window::WindowId, Arc<RwLock<Viewport>>>,
    pub pipelines: HashMap<String, wgpu::RenderPipeline>,
    pub shaders: HashMap<String, Asset>,

    commands: Vec<Command>,
}

impl Sun {
    pub async fn create_adapter(&mut self, surface: &wgpu::Surface) {
        let adapter = self
            .instance
            .as_ref()
            .unwrap()
            .request_adapter(&wgpu::RequestAdapterOptions {
                // Request an adapter which can render to our surface
                compatible_surface: Some(surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find an appropriate adapter");
        self.adapter = Some(adapter);
    }

    pub async fn create_device(&mut self) {
        // Create the logical device and command queue
        let (device, queue) = self
            .adapter
            .as_ref()
            .unwrap()
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        self.device = Some(Arc::new(device));
        self.queue = Some(queue);
    }

    pub async fn create_viewport(&mut self, window: Arc<Window>) {
        let vp_desc = ViewportDesc::new(
            Arc::clone(&window),
            wgpu::Color {
                r: 105.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            self.instance.as_ref().unwrap(),
        );

        if self.adapter.is_none() {
            self.create_adapter(&vp_desc.surface).await;
        }
        if self.device.is_none() {
            self.create_device().await;
        }

        let vp = vp_desc.build(
            self.adapter.as_ref().unwrap(),
            self.device.as_ref().unwrap(),
        );

        self.viewports
            .insert(window.id(), Arc::new(RwLock::new(vp)));
    }

    pub async fn create_pipeline(
        &mut self,
        win_id: winit::window::WindowId,
        name: String,
        shader_src: impl AsRef<str>,
    ) {
        let vp = self.viewports.get(&win_id).unwrap();

        let shader =
            self.device
                .as_ref()
                .unwrap()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&name),
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(shader_src.as_ref())),
                });

        let layout =
            self.device
                .as_ref()
                .unwrap()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&name),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .as_ref()
                .unwrap()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                            format: vp.read().await.get_config().format,
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

        self.pipelines.insert(name, pipeline);
    }

    pub async fn redraw(&mut self, render_desc: RenderDesc) {
        if let Some(viewport) = self.viewports.get_mut(&render_desc.window_id) {
            let mut vp = viewport.write().await;

            let frame = vp.get_current_texture();
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .as_ref()
                .unwrap()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let test_pp = self.pipelines.get("basic_shader.wgsl");

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(vp.desc.background),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                if let Some(pp) = test_pp {
                    rpass.set_pipeline(pp);
                    rpass.draw(0..3, 0..1);
                }
            }

            self.queue.as_ref().unwrap().submit(Some(encoder.finish()));
            frame.present();
        }
    }
}

impl Default for Sun {
    fn default() -> Self {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            dx12_shader_compiler: Default::default(),
            ..Default::default()
        });

        Self {
            instance: Some(instance),
            adapter: None,
            device: None,
            queue: None,

            viewports: HashMap::new(),
            pipelines: HashMap::new(),
            shaders: HashMap::new(),

            commands: vec![],
        }
    }
}

#[async_trait(?Send)]
impl App for Sun {
    fn init(&mut self, mut init_commands: Vec<crate::core::command_queue::Command>) {
        self.commands.append(&mut init_commands);
    }

    fn process_command(&mut self, _cmd: Command) {}

    async fn process_event(
        &mut self,
        event: &winit::event::Event<crate::core::events::CommandEvent>,
        elp: EventLoopProxy<CommandEvent>,
    ) {
        if let Event::UserEvent(event) = event {
            match event {
                CommandEvent::RequestSurface(window) => {
                    self.create_viewport(Arc::clone(window)).await;
                }
                CommandEvent::CloseWindow((id, _)) => {
                    self.viewports.remove(id);
                }

                CommandEvent::Render(render_desc) => {
                    self.redraw(render_desc.clone()).await;
                }

                CommandEvent::RequestPipeline(pipe_desc) => {
                    let pipe_desc = pipe_desc.clone();

                    self.create_pipeline(pipe_desc.win_id, pipe_desc.name, pipe_desc.shader_src)
                        .await;
                }

                CommandEvent::Asset(asset) => {
                    if asset.asset_type == AssetType::Shader {
                        info!("Asset!");
                        self.shaders.insert(asset.name.clone(), asset.clone());
                    }
                }
                _ => {}
            }
        }

        if let Event::WindowEvent { window_id, event } = event {
            match event {
                WindowEvent::Resized(new_size) => {
                    // Recreate the swap chain with the new size
                    if let Some(viewport) = self.viewports.get_mut(window_id) {
                        {
                            viewport
                                .write()
                                .await
                                .resize(self.device.as_ref().unwrap(), new_size);
                        }
                        // On macos the window needs to be redrawn manually after resizing
                        viewport.read().await.desc.window.request_redraw();
                    }
                }
                WindowEvent::CloseRequested => {
                    self.viewports.remove(window_id);
                }

                WindowEvent::RedrawRequested => {
                    for (name, shader) in &self.shaders {
                        if !self.pipelines.contains_key(name) {
                            let pipe_desc = PipelineDesc {
                                name: name.clone(),
                                win_id: *window_id,
                                shader_src: shader.data.clone(),
                            };

                            elp.send_event(CommandEvent::RequestPipeline(pipe_desc))
                                .unwrap();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct ViewportDesc {
    window: Arc<Window>,
    background: wgpu::Color,
    surface: wgpu::Surface,
}

#[derive(Debug)]
pub struct Viewport {
    desc: ViewportDesc,
    config: wgpu::SurfaceConfiguration,
}

impl ViewportDesc {
    fn new(window: Arc<Window>, background: wgpu::Color, instance: &wgpu::Instance) -> Self {
        let surface = unsafe { instance.create_surface(window.clone().as_ref()).unwrap() };
        Self {
            window,
            background,
            surface,
        }
    }

    fn build(self, adapter: &wgpu::Adapter, device: &wgpu::Device) -> Viewport {
        let size = self.window.inner_size();

        let caps = self.surface.get_capabilities(adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };

        self.surface.configure(device, &config);

        Viewport { desc: self, config }
    }
}

impl Viewport {
    fn resize(&mut self, device: &wgpu::Device, size: &winit::dpi::PhysicalSize<u32>) {
        if size.height != 0 && size.width != 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.desc.surface.configure(device, &self.config);
        }
    }
    fn get_current_texture(&mut self) -> wgpu::SurfaceTexture {
        self.desc
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture")
    }

    pub fn get_surface(&self) -> &wgpu::Surface {
        &self.desc.surface
    }

    pub fn get_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }
}
