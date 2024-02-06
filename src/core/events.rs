use std::sync::Arc;

use winit::dpi::PhysicalSize;

use crate::prelude::{
    sun::{BufferDesc, PipelineDesc, RenderDesc},
    Asset,
};

#[derive(Default, Debug, Clone)]
pub struct NewWindowProps {
    pub size: PhysicalSize<u32>,
    pub name: String,
    pub element_id: String,
    // Option<Decorations...icon...etc, etc>
}

#[derive(Debug, Clone)]
pub enum CommandEvent {
    OpenWindow(NewWindowProps),
    CloseWindow((winit::window::WindowId, String)),
    RequestSurface(Arc<winit::window::Window>),
    RequestPipeline(PipelineDesc),
    RequestCreateBuffer(BufferDesc),
    RequestDestroyBuffer(uuid::Uuid),
    Render(RenderDesc),
    Asset(Asset),
    Exit,
    // File(String),
    // FileNotFound,
    // FilePending(String),
    None,
}
