use std::{collections::HashMap, sync::Arc};

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
    prelude::{
        camera_component::{CamType, CameraComponent, CameraController, PerspectiveProps},
        command_queue::CommandType,
        state, Asset, AssetType,
    },
};

pub type ResourceID = uuid::Uuid;
pub type PrimitiveID = uuid::Uuid;

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
    pub bind_group_layout_descs: Vec<wgpu::BindGroupLayoutDescriptor<'static>>,
    pub topology: wgpu::PrimitiveTopology,
}

use super::{
    buffer::{CameraUniform, SunBuffer},
    pipeline::SunPipeline,
    primitive::Primitive,
    texture::GPUTexture,
};

pub struct Sun {
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,

    pub shaders: HashMap<String, Asset>,

    pub viewports: HashMap<winit::window::WindowId, Viewport>,
    pub pipelines: HashMap<String, SunPipeline>,
    pub vertex_buffers: HashMap<PrimitiveID, SunBuffer>,
    pub index_buffers: HashMap<PrimitiveID, SunBuffer>,

    pub test_texture: Option<GPUTexture>,

    // This needs to go in model
    pub texture_ids: HashMap<String, ResourceID>,

    // This needs to be querried from the scene
    pub camera_controller: CameraController,
    pub current_camera: Option<CameraComponent>,
    pub current_camera_buffer: Option<SunBuffer>,
    pub current_camera_uniform: Option<CameraUniform>,

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

        self.viewports.insert(window.id(), vp);
    }

    pub async fn create_pipeline(
        &mut self,
        win_id: winit::window::WindowId,
        name: String,
        shader_src: impl AsRef<str>,
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'static>],
        bind_group_layout_descs: Vec<wgpu::BindGroupLayoutDescriptor<'static>>,
        topology: wgpu::PrimitiveTopology,
    ) {
        let viewport = self.viewports.get(&win_id).unwrap();
        let mut pipeline = SunPipeline::new(
            self.device.as_ref().unwrap(),
            viewport,
            name.clone(),
            shader_src,
            vertex_buffer_layouts,
            bind_group_layout_descs,
            topology,
        );

        if self.current_camera.is_none() && name == "basic_shader.wgsl" {
            let camera = CameraComponent {
                camera_type: CamType::Perspective(PerspectiveProps {
                    aspect: viewport.config.width as f32 / viewport.config.height as f32,
                    fovy: 45.0,
                }),
                // position the camera 1 unit up and 2 units back
                // +z is out of the screen
                eye: (0.0, 1.0, 2.0).into(),
                // have it look at the origin
                target: (0.0, 0.0, 0.0).into(),
                // which way is "up"
                up: cgmath::Vector3::unit_y(),

                znear: 0.1,
                zfar: 100.0,
                uuid: uuid::Uuid::new_v4(),
            };

            let mut camera_uniform = CameraUniform::new();
            camera_uniform.update_view_proj(&camera);

            let camera_buffer = SunBuffer::new_with_data(
                "Camera Uniform Buffer",
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                bytemuck::cast_slice(&[camera_uniform]),
                self.device.as_ref().unwrap(),
            );

            pipeline.add_bind_group(
                self.device.as_ref().unwrap(),
                camera.uuid,
                "camera",
                1,
                &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.get_buffer().as_entire_binding(),
                }],
            );
            self.current_camera = Some(camera);
            self.current_camera_buffer = Some(camera_buffer);
            self.current_camera_uniform = Some(camera_uniform);
        }
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

    pub fn destroy_buffer(&mut self, id: PrimitiveID) {
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
        if let Some(vp) = self.viewports.get_mut(&render_desc.window_id) {
            let frame = vp.get_current_texture();
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .as_ref()
                .unwrap()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let pipeline_entry = if self.lined {
                self.pipelines.get_key_value("line_shader.wgsl")
            } else {
                self.pipelines.get_key_value("basic_shader.wgsl")
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
                    if let Some((name, pipeline)) = pipeline_entry {
                        rpass.set_pipeline(&pipeline.pipeline);
                        if let Some(cam) = &self.current_camera {
                            if name != "line_shader.wgsl" {
                                // Diffuse texture bind group (set to default missing texture if not present on the primitive)
                                if let Some(tex_name) = &primitive.temp_diffuse {
                                    let tex_id = self.texture_ids.get(tex_name).unwrap();
                                    let bind_group = pipeline.bind_groups.get(tex_id).unwrap();

                                    rpass.set_bind_group(bind_group.0, &bind_group.1, &[]);
                                } else {
                                    let tex_id = self.texture_ids.get("missing.jpg").unwrap();
                                    let bind_group = pipeline.bind_groups.get(tex_id).unwrap();

                                    rpass.set_bind_group(bind_group.0, &bind_group.1, &[]);
                                }

                                let cam_bg = pipeline.bind_groups.get(&cam.uuid).unwrap();
                                rpass.set_bind_group(cam_bg.0, &cam_bg.1, &[]);
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

            shaders: HashMap::new(),
            viewports: HashMap::new(),
            pipelines: HashMap::new(),
            vertex_buffers: HashMap::new(),
            index_buffers: HashMap::new(),

            test_texture: None,

            texture_ids: HashMap::new(),

            camera_controller: CameraController::new(0.2),
            current_camera: None,
            current_camera_buffer: None,
            current_camera_uniform: None,

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

        self.commands.push(load_basic_shader);

        let load_line_shader = Command::new(
            "asset_server",
            CommandType::Get,
            Some("get shaders/line_shader.wgsl shader".into()),
            None,
        );

        self.commands.push(load_line_shader);
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
                        pipe_desc.bind_group_layout_descs,
                        pipe_desc.topology,
                    )
                    .await;
                }

                CommandEvent::Asset(asset) => {
                    if asset.asset_type == AssetType::Shader {
                        self.shaders.insert(asset.name.clone(), asset.clone());
                    } else if asset.asset_type == AssetType::Texture {
                        let asset = asset.clone();
                        self.test_texture = Some(
                            GPUTexture::from_bytes(
                                self.device.as_ref().unwrap(),
                                self.queue.as_ref().unwrap(),
                                asset.data.as_slice(),
                                asset.name.as_str(),
                            )
                            .unwrap(),
                        );

                        self.pipelines
                            .get_mut("basic_shader.wgsl")
                            .unwrap()
                            .add_bind_group(
                                self.device.as_ref().unwrap(),
                                self.test_texture.as_ref().unwrap().uuid,
                                "diffuse",
                                0,
                                &[
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
                            );

                        self.texture_ids.insert(
                            self.test_texture.as_ref().unwrap().name.clone(),
                            self.test_texture.as_ref().unwrap().uuid,
                        );
                    }
                }

                CommandEvent::RequestCreateBuffer(desc) => self.generate_buffers(desc).await,
                CommandEvent::RequestDestroyBuffer(id) => self.destroy_buffer(id.clone()),
                _ => {}
            }
        }

        if let Event::WindowEvent { window_id, event } = event {
            self.camera_controller.process_events(event);

            match event {
                WindowEvent::Resized(new_size) => {
                    // Recreate the swap chain with the new size
                    if let Some(viewport) = self.viewports.get_mut(window_id) {
                        {
                            viewport.resize(self.device.as_ref().unwrap(), new_size);
                        }
                        // On macos the window needs to be redrawn manually after resizing
                        viewport.desc.window.request_redraw();
                    }
                }
                WindowEvent::CloseRequested => {
                    self.viewports.remove(window_id);
                }

                WindowEvent::RedrawRequested => {
                    let basic_tex_bg_layout_desc =
                        GPUTexture::layout_desc("diffuse_texture_bind_group_layout");

                    let camera_bg_layout_decs = CameraComponent::layout_desc();

                    for (name, shader) in &self.shaders {
                        if !self.pipelines.contains_key(name) {
                            let pipe_desc = PipelineDesc {
                                name: name.clone(),
                                win_id: *window_id,
                                shader_src: std::str::from_utf8(shader.data.clone().as_slice())
                                    .unwrap()
                                    .to_owned(),
                                vertex_buffer_layouts: vec![super::primitive::Vertex::desc()],
                                topology: if name != "basic_shader.wgsl" {
                                    wgpu::PrimitiveTopology::LineList
                                } else {
                                    wgpu::PrimitiveTopology::TriangleList
                                },
                                bind_group_layout_descs: if name != "basic_shader.wgsl" {
                                    vec![]
                                } else {
                                    vec![
                                        basic_tex_bg_layout_desc.clone(),
                                        camera_bg_layout_decs.clone(),
                                    ]
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
                            vp.desc.window.request_redraw();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        if let Some(cam) = self.current_camera.as_mut() {
            self.camera_controller.update_camera(cam);

            let mut camera_uniform = CameraUniform::new();
            camera_uniform.update_view_proj(&cam);

            self.current_camera_uniform.unwrap().update_view_proj(&cam);
            self.queue.as_ref().unwrap().write_buffer(
                self.current_camera_buffer.as_ref().unwrap().get_buffer(),
                0,
                bytemuck::cast_slice(&[camera_uniform]),
            );
        }

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
pub struct ViewportDesc {
    pub window: Arc<Window>,
    pub background: wgpu::Color,
    pub surface: wgpu::Surface<'static>,
}

#[derive(Debug)]
pub struct Viewport {
    pub desc: ViewportDesc,
    pub config: wgpu::SurfaceConfiguration,
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
