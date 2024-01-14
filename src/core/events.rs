use std::{collections::HashMap, sync::Arc};

use async_std::sync::RwLock;
use winit::dpi::PhysicalSize;

use crate::renderer::sun::Viewport;

#[derive(Default, Debug, Clone)]
pub struct NewWindowProps {
    pub size: PhysicalSize<u32>,
    pub name: String,
    // Option<Decorations...icon...etc, etc>
}

#[derive(Debug, Clone)]
pub struct PipelineDesc {
    pub name: String,
    pub device: Arc<wgpu::Device>,
    pub viewport: Arc<RwLock<Viewport>>,
    pub shader_src: String,
}

#[derive(Debug, Clone)]
pub struct RenderDesc {
    pub world: Arc<RwLock<bevy_ecs::world::World>>,
    pub pipelines: Arc<RwLock<HashMap<String, wgpu::RenderPipeline>>>,
    pub window_id: winit::window::WindowId,
}

#[derive(Debug, Clone)]
pub enum CommandEvent {
    OpenWindow(NewWindowProps),
    CloseWindow((winit::window::WindowId, String)),
    RequestSurface(Arc<winit::window::Window>),
    RequestPipeline(PipelineDesc),
    Render(RenderDesc),
    Exit,
    // File(String),
    // FileNotFound,
    // FilePending(String),
    None,
}
