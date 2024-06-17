use std::sync::Arc;

use crate::prelude::{
    model_component::ModelComponent, pipeline::PipelineDesc, sun::RenderFrameDesc,
    windower::NewWindowProps, Asset, ChangeComponentState,
};

#[derive(Clone, bevy_ecs::event::Event)]
pub enum CommandEvent {
    RequestNewWindow(NewWindowProps),
    OnWindowClosed((winit::window::WindowId, String)),
    // TODO: (@A40) Add VP desc to the event
    OnWindowCreated(Arc<winit::window::Window>),

    RequestPipeline(PipelineDesc),
    RequestDestroyBuffer(uuid::Uuid),
    RenderFrame(RenderFrameDesc),

    Asset(Asset),
    RequestCreateModel(ModelComponent),
    ChangedAssets(Vec<String>),

    SignalChange(ChangeComponentState),

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
            CommandEvent::RequestDestroyBuffer(id) => {
                write!(f, "Event <RequestCreateBuffer> with: {id:?}")
            }
            CommandEvent::RenderFrame(_) => write!(f, "Event <RenderFrame>"),
            CommandEvent::Asset(asset) => write!(f, "Event <Asset> with: {asset:?}"),
            CommandEvent::Exit => write!(f, "Event <Exit>"),
            CommandEvent::None => write!(f, "Event <None>"),
            CommandEvent::RequestCreateModel(model_comp) => {
                write!(f, "Event <RequestCreateModel> with: {model_comp:?}")
            }
            CommandEvent::ChangedAssets(paths) => {
                write!(f, "Event <ChangedAssets> with: {paths:?}")
            }
            CommandEvent::SignalChange(e) => {
                write!(f, "Event <SignalChange> with: {e:?}")
            }
        }
    }
}
