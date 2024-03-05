use std::sync::Arc;

use winit::dpi::PhysicalSize;

use crate::prelude::{buffer::BufferDesc, pipeline::PipelineDesc, sun::RenderDesc, Asset};

#[derive(Default, Debug, Clone)]
pub struct NewWindowProps {
    pub size: PhysicalSize<u32>,
    pub name: String,
    pub element_id: String,
    // Option<Decorations...icon...etc, etc>
}

#[derive(Clone)]
pub enum CommandEvent {
    RequestNewWindow(NewWindowProps),
    OnWindowClosed((winit::window::WindowId, String)),
    // TODO: (@A40) Add VP desc to the event
    OnWindowCreated(Arc<winit::window::Window>),

    RequestPipeline(PipelineDesc),
    RequestCreateBuffer(BufferDesc),
    RequestDestroyBuffer(uuid::Uuid),
    Render(RenderDesc),
    Asset(Asset),
    Exit,
    None,
}

impl std::fmt::Debug for CommandEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandEvent::RequestNewWindow(props) => write!(f, "Event <NewWindow> with: {props:?}"),
            CommandEvent::OnWindowClosed((id, name)) => {
                write!(f, "Event <WindowClosed> with: {id:?}, {name:?}")
            }
            CommandEvent::OnWindowCreated(_) => write!(f, "Event <OnWindowCreated>"),
            CommandEvent::RequestPipeline(props) => {
                write!(f, "Event <RequestPipeline> with: {props:?}")
            }
            CommandEvent::RequestCreateBuffer(desc) => {
                write!(f, "Event <RequestCreateBuffer> with: {desc:?}")
            }
            CommandEvent::RequestDestroyBuffer(id) => {
                write!(f, "Event <RequestCreateBuffer> with: {id:?}")
            }
            CommandEvent::Render(_) => write!(f, "Event <Render>"),
            CommandEvent::Asset(asset) => write!(f, "Event <Asset> with: {asset:?}"),
            CommandEvent::Exit => write!(f, "Event <Exit>"),
            CommandEvent::None => write!(f, "Event <None>"),
        }
    }
}
