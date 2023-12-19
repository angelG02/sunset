use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use wgpu::{Adapter, Device, Instance, Queue};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopProxy,
    window::Window,
};

use crate::core::{app::App, command_queue::Command, events::CommandEvent};

pub struct Sun {
    instance: Arc<Option<wgpu::Instance>>,
    adapter: Arc<Option<wgpu::Adapter>>,
    device: Arc<Option<wgpu::Device>>,
    queue: Arc<Option<wgpu::Queue>>,

    pub viewports: HashMap<winit::window::WindowId, Arc<Mutex<Viewport>>>,

    commands: Vec<Command>,
}

unsafe impl Send for Sun {}
unsafe impl Sync for Sun {}

impl Sun {
    pub async fn create_adapter(&mut self, surface: &wgpu::Surface) {
        let adapter = self
            .get_instance()
            .unwrap()
            .request_adapter(&wgpu::RequestAdapterOptions {
                // Request an adapter which can render to our surface
                compatible_surface: Some(surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find an appropriate adapter");
        self.adapter = Arc::new(Some(adapter));
    }

    pub async fn create_device(&mut self) {
        // Create the logical device and command queue
        let (device, queue) = self
            .get_adapter()
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

        self.device = Arc::new(Some(device));
        self.queue = Arc::new(Some(queue));
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
            self.get_instance().unwrap(),
        );

        if self.adapter.is_none() {
            self.create_adapter(&vp_desc.surface).await;
        }
        if self.device.is_none() {
            self.create_device().await;
        }

        let vp = vp_desc.build(Arc::clone(&self.adapter), Arc::clone(&self.device));

        self.viewports.insert(window.id(), Arc::new(Mutex::new(vp)));
    }

    pub fn clear_screen(&mut self, window_id: &winit::window::WindowId) {
        if let Some(viewport) = self.viewports.get_mut(window_id) {
            let mut vp_lock = viewport.lock().unwrap();
            let frame = vp_lock.get_current_texture();
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .as_ref()
                .as_ref()
                .unwrap()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(vp_lock.desc.background),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }

            self.queue
                .as_ref()
                .as_ref()
                .unwrap()
                .submit(Some(encoder.finish()));
            frame.present();
        }
    }

    pub fn get_instance(&self) -> Option<&Instance> {
        self.instance.as_ref().as_ref()
    }

    pub fn get_adapter(&self) -> Option<&Adapter> {
        self.adapter.as_ref().as_ref()
    }

    pub fn get_device(&self) -> Option<&Device> {
        self.device.as_ref().as_ref()
    }

    pub fn get_queue(&self) -> Option<&Queue> {
        self.queue.as_ref().as_ref()
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
            instance: Arc::new(Some(instance)),
            adapter: Arc::new(None),
            device: Arc::new(None),
            queue: Arc::new(None),

            viewports: HashMap::new(),
            commands: vec![],
        }
    }
}

#[async_trait]
impl App for Sun {
    fn init(&mut self, mut init_commands: Vec<crate::core::command_queue::Command>) {
        self.commands.append(&mut init_commands);
    }

    fn process_command(&mut self, _cmd: Command) {}

    async fn process_event(
        &mut self,
        event: &winit::event::Event<crate::core::events::CommandEvent>,
        _elp: EventLoopProxy<CommandEvent>,
    ) {
        if let Event::UserEvent(CommandEvent::RequestSurface(window)) = event {
            self.create_viewport(Arc::clone(window)).await;
        }
        if let Event::UserEvent(CommandEvent::CloseWindow((id, _))) = event {
            self.viewports.remove(id);
        }
        if let Event::WindowEvent { window_id, event } = event {
            match event {
                WindowEvent::Resized(new_size) => {
                    // Recreate the swap chain with the new size
                    if let Some(viewport) = self.viewports.get_mut(window_id) {
                        let mut vp_lock = viewport.lock().unwrap();
                        vp_lock.resize(self.device.as_ref().as_ref().unwrap(), *new_size);
                        // On macos the window needs to be redrawn manually after resizing
                        vp_lock.desc.window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => self.clear_screen(window_id),
                WindowEvent::CloseRequested => {
                    self.viewports.remove(window_id);
                }
                _ => {}
            }
        }
    }

    fn queue_commands(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

struct ViewportDesc {
    window: Arc<Window>,
    background: wgpu::Color,
    surface: wgpu::Surface,
}

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

    fn build(
        self,
        adapter: Arc<Option<wgpu::Adapter>>,
        device: Arc<Option<wgpu::Device>>,
    ) -> Viewport {
        let size = self.window.inner_size();

        let caps = self
            .surface
            .get_capabilities(adapter.as_ref().as_ref().unwrap());
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };

        self.surface
            .configure(device.as_ref().as_ref().unwrap(), &config);

        Viewport { desc: self, config }
    }
}

impl Viewport {
    fn resize(&mut self, device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) {
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
}
