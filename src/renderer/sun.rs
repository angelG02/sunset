use std::{collections::HashMap, sync::Arc};

use async_std::sync::RwLock;
use async_trait::async_trait;
use tracing::info;
use wgpu::BufferUsages;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopProxy,
    keyboard::PhysicalKey,
    window::Window,
};

use crate::{
    core::{app::App, command_queue::Command, events::CommandEvent},
    prelude::{command_queue::CommandType, state, Asset, AssetType},
};

#[derive(Debug, Clone)]
pub struct RenderDesc {
    pub primitives: Vec<Primitive>,
    pub window_id: winit::window::WindowId,
}

#[derive(Debug, Clone)]
pub struct BufferDesc {
    pub data: Vec<Primitive>,
}

#[derive(Debug, Clone)]
pub struct PipelineDesc {
    pub name: String,
    pub win_id: winit::window::WindowId,
    pub shader_src: String,
    pub vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    pub bind_group_layout_desc: Option<wgpu::BindGroupLayoutDescriptor<'static>>,
    pub bind_group_layout_name: Option<String>,
    pub topology: wgpu::PrimitiveTopology,
}

use super::{buffer::SunBuffer, primitive::Primitive, texture::GPUTexture};

pub struct Sun {
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,

    pub viewports: HashMap<winit::window::WindowId, Arc<RwLock<Viewport>>>,
    pub pipelines: HashMap<String, wgpu::RenderPipeline>,
    pub shaders: HashMap<String, Asset>,
    pub vertex_buffers: HashMap<uuid::Uuid, SunBuffer>,
    pub index_buffers: HashMap<uuid::Uuid, SunBuffer>,

    pub test_texture: Option<GPUTexture>,

    pub bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,
    pub bind_groups: HashMap<uuid::Uuid, wgpu::BindGroup>,

    // This needs to go in model
    pub texture_ids: Vec<uuid::Uuid>,

    pub lined: bool,

    commands: Vec<Command>,

    pub proxy: Option<EventLoopProxy<CommandEvent>>,
}

impl Sun {
    pub async fn create_adapter(&mut self, surface: &wgpu::Surface<'static>) {
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
                    required_features: wgpu::Features::empty(),
                    required_limits: self.adapter.as_ref().unwrap().limits(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        self.device = Some(device);
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
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'static>],
        bind_group_layout_desc: Option<wgpu::BindGroupLayoutDescriptor<'static>>,
        bind_group_layout_name: Option<String>,
        topology: wgpu::PrimitiveTopology,
    ) {
        if let Some(bgld) = bind_group_layout_desc.clone() {
            let texture_bind_group_layout = Some(
                self.device
                    .as_ref()
                    .unwrap()
                    .create_bind_group_layout(&bgld),
            );
            self.bind_group_layouts.insert(
                bind_group_layout_name.unwrap(),
                texture_bind_group_layout.unwrap(),
            );
        }

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
                    bind_group_layouts: self
                        .bind_group_layouts
                        .values()
                        .collect::<Vec<&_>>()
                        .as_slice(),
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
                        buffers: vertex_buffer_layouts,
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

        self.pipelines.insert(name, pipeline);

        if !state::initialized() {
            state::finish_init();
            info!("Initialized Render Engine!")
        }
    }

    pub async fn generate_buffers(&mut self, buf_desc: &BufferDesc) {
        for primitive in &buf_desc.data {
            if !primitive.initialized {
                let vb = SunBuffer::new_with_data(
                    format!("Vertex Buffer: {}", primitive.uuid).as_str(),
                    BufferUsages::VERTEX,
                    bytemuck::cast_slice(primitive.vertices.as_slice()),
                    self.device.as_ref().unwrap(),
                );

                if !primitive.indices.is_empty() {
                    let ib = SunBuffer::new_with_data(
                        format!("Index Buffer: {}", primitive.uuid).as_str(),
                        BufferUsages::INDEX,
                        bytemuck::cast_slice(primitive.indices.as_slice()),
                        self.device.as_ref().unwrap(),
                    );
                    self.index_buffers.insert(primitive.uuid, ib);
                }

                self.vertex_buffers.insert(primitive.uuid, vb);
            }
        }
    }

    pub fn destroy_buffer(&mut self, id: uuid::Uuid) {
        if self.vertex_buffers.contains_key(&id) {
            self.vertex_buffers
                .remove(&id)
                .unwrap()
                .get_buffer()
                .destroy();
        }
        if self.index_buffers.contains_key(&id) {
            self.index_buffers
                .remove(&id)
                .unwrap()
                .get_buffer()
                .destroy();
        }
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

            let test_pp = if self.lined {
                self.pipelines.get("line_shader.wgsl")
            } else {
                self.pipelines.get("basic_shader.wgsl")
            };

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

                for primitive in &render_desc.primitives {
                    if let Some(pp) = test_pp {
                        rpass.set_pipeline(pp);

                        for i in 0..self.texture_ids.len() {
                            rpass.set_bind_group(
                                i as u32,
                                self.bind_groups
                                    .get(self.texture_ids.get(i).unwrap())
                                    .unwrap(),
                                &[],
                            );
                        }

                        if let Some(vb) = self.vertex_buffers.get(&primitive.uuid) {
                            rpass.set_vertex_buffer(0, vb.get_buffer().slice(..));
                        }

                        if let Some(ib) = self.index_buffers.get(&primitive.uuid) {
                            rpass.set_index_buffer(
                                ib.get_buffer().slice(..),
                                wgpu::IndexFormat::Uint16,
                            )
                        }

                        rpass.draw_indexed(0..primitive.indices.len() as u32, 0, 0..1);
                    }
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
            vertex_buffers: HashMap::new(),
            index_buffers: HashMap::new(),

            test_texture: None,
            bind_groups: HashMap::new(),
            bind_group_layouts: HashMap::new(),

            texture_ids: Vec::new(),

            lined: false,

            commands: vec![],

            proxy: None,
        }
    }
}

#[async_trait(?Send)]
impl App for Sun {
    fn init(&mut self, elp: EventLoopProxy<CommandEvent>) {
        self.proxy = Some(elp.clone());

        let load_basic_shader = Command::new(
            "asset_server",
            CommandType::Get,
            Some("get shaders/basic_shader.wgsl shader".into()),
            None,
        );

        let load_line_shader = Command::new(
            "asset_server",
            CommandType::Get,
            Some("get shaders/line_shader.wgsl shader".into()),
            None,
        );

        self.commands
            .append(&mut vec![load_basic_shader, load_line_shader]);
    }

    async fn process_command(&mut self, _cmd: Command) {}

    async fn process_event(
        &mut self,
        event: &winit::event::Event<crate::core::events::CommandEvent>,
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

                    self.create_pipeline(
                        pipe_desc.win_id,
                        pipe_desc.name,
                        pipe_desc.shader_src,
                        &pipe_desc.vertex_buffer_layouts,
                        pipe_desc.bind_group_layout_desc,
                        pipe_desc.bind_group_layout_name,
                        pipe_desc.topology,
                    )
                    .await;
                }

                CommandEvent::Asset(asset) => {
                    if asset.asset_type == AssetType::Shader {
                        self.shaders.insert(asset.name.clone(), asset.clone());
                    } else if asset.asset_type == AssetType::Texture {
                        self.test_texture = Some(
                            GPUTexture::from_bytes(
                                self.device.as_ref().unwrap(),
                                self.queue.as_ref().unwrap(),
                                asset.data.as_slice(),
                                asset.name.as_str(),
                            )
                            .unwrap(),
                        );

                        self.texture_ids
                            .push(self.test_texture.as_ref().unwrap().uuid);

                        let test_diffuse_bind_group = self
                            .device
                            .as_ref()
                            .unwrap()
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some(&asset.name),
                                layout: self.bind_group_layouts.get("basic_shader.wgsl").unwrap(),
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(
                                            &self.test_texture.as_ref().unwrap().view,
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(
                                            &self.test_texture.as_ref().unwrap().sampler,
                                        ),
                                    },
                                ],
                            });

                        self.bind_groups.insert(
                            self.test_texture.as_ref().unwrap().uuid,
                            test_diffuse_bind_group,
                        );
                    }
                }

                CommandEvent::RequestCreateBuffer(desc) => self.generate_buffers(desc).await,
                CommandEvent::RequestDestroyBuffer(id) => self.destroy_buffer(id.clone()),
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
                    let line_bg_layout_desc = None;
                    let basic_tex_bg_layout_desc = Some(wgpu::BindGroupLayoutDescriptor {
                        label: Some("Diffuse Tex Bind Group Description"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                // This should match the filterable field of the
                                // corresponding Texture entry above.
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                    });

                    for (name, shader) in &self.shaders {
                        if !self.pipelines.contains_key(name) {
                            let pipe_desc = PipelineDesc {
                                name: name.clone(),
                                win_id: *window_id,
                                shader_src: std::str::from_utf8(shader.data.clone().as_slice())
                                    .unwrap()
                                    .to_owned(),
                                vertex_buffer_layouts: vec![super::primitive::Vertex::desc()],
                                topology: if name.contains("line") {
                                    wgpu::PrimitiveTopology::LineList
                                } else {
                                    wgpu::PrimitiveTopology::TriangleList
                                },
                                bind_group_layout_desc: if name.contains("line") {
                                    line_bg_layout_desc.clone()
                                } else {
                                    basic_tex_bg_layout_desc.clone()
                                },
                                bind_group_layout_name: if name.contains("line") {
                                    None
                                } else {
                                    Some(name.clone())
                                },
                            };

                            self.proxy
                                .as_ref()
                                .unwrap()
                                .send_event(CommandEvent::RequestPipeline(pipe_desc))
                                .unwrap();
                        }
                    }
                }
                WindowEvent::KeyboardInput {
                    device_id: _,
                    event,
                    is_synthetic: _,
                } => {
                    if event.physical_key == PhysicalKey::Code(winit::keyboard::KeyCode::F1)
                        && event.state == winit::event::ElementState::Released
                    {
                        self.lined = !self.lined;
                        for vp in self.viewports.values() {
                            vp.read().await.desc.window.request_redraw();
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
    surface: wgpu::Surface<'static>,
}

#[derive(Debug)]
pub struct Viewport {
    desc: ViewportDesc,
    config: wgpu::SurfaceConfiguration,
}

impl ViewportDesc {
    fn new(window: Arc<Window>, background: wgpu::Color, instance: &wgpu::Instance) -> Self {
        let surface = unsafe {
            instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(window.clone().as_ref()).unwrap(),
                )
                .unwrap()
        };
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
            desired_maximum_frame_latency: 3,
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
