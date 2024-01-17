use async_std::sync::RwLock;
use async_trait::async_trait;
use tracing::info;
use winit::event_loop::EventLoopProxy;

use crate::{
    core::{
        app::App,
        command_queue::{Command, CommandType},
        events::CommandEvent,
    },
    prelude::{asset_cmd::AssetCommand, AssetType},
};
use std::sync::Arc;

#[derive(Default)]
pub struct Scene {
    pub world: Arc<RwLock<bevy_ecs::world::World>>,
    pub commands: Vec<Command>,
}

impl Scene {
    pub fn new() -> Self {
        Scene::default()
    }
}

#[async_trait(?Send)]
impl App for Scene {
    fn init(&mut self, elp: EventLoopProxy<CommandEvent>) {
        let load_basic_shader = Command::new(
            "default_scene",
            CommandType::Get,
            None,
            AssetCommand::get_from_server(
                "127.0.0.1:7878 shaders/basic_shader.wgsl shader".into(),
                elp.clone(),
            )
            .unwrap(),
        );

        let load_test_tex = Command::new(
            "default_scene",
            CommandType::Get,
            None,
            AssetCommand::get_from_server(
                "127.0.0.1:7878 textures/happy-tree.png texture".into(),
                elp.clone(),
            )
            .unwrap(),
        );

        self.commands
            .append(&mut vec![load_basic_shader, load_test_tex]);
    }

    fn process_command(&mut self, _cmd: Command) {}

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    async fn process_event(
        &mut self,
        event: &winit::event::Event<crate::core::events::CommandEvent>,
        elp: EventLoopProxy<CommandEvent>,
    ) {
        #[allow(clippy::single_match)]
        match event {
            winit::event::Event::WindowEvent {
                window_id,
                event: winit::event::WindowEvent::RedrawRequested,
            } => {
                use crate::core::events::RenderDesc;

                let render_desc = RenderDesc {
                    world: self.world.clone(),
                    window_id: *window_id,
                };
                elp.send_event(CommandEvent::Render(render_desc)).unwrap();
            }
            winit::event::Event::UserEvent(event) => {
                if let CommandEvent::Asset(asset) = event {
                    if asset.asset_type == AssetType::Texture {
                        info!("Yuppee");
                    }
                }
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
