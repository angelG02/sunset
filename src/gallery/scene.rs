use async_trait::async_trait;
use tracing::{error, info};
use winit::event_loop::EventLoopProxy;

use crate::{
    core::{
        app::App,
        command_queue::{Command, CommandType, Task},
        events::CommandEvent,
    },
    prelude::{name_component, primitive::Primitive, sun::RenderDesc, AssetType},
};

#[derive(Default)]
pub struct Scene {
    pub world: bevy_ecs::world::World,
    pub commands: Vec<Command>,

    pub proxy: Option<EventLoopProxy<CommandEvent>>,
}

impl Scene {
    pub fn new() -> Self {
        Scene::default()
    }

    pub async fn process_scene_commands(&mut self, mut cmd: Command) {
        let args = cmd.args.clone().unwrap();
        let vec_args: Vec<&str> = args.split(' ').collect();

        let task = match vec_args[0].to_ascii_lowercase().trim() {
            "add" => {
                self.add_entity(
                    vec_args[1..].join(" ").as_str(),
                    self.proxy.as_ref().unwrap().clone(),
                )
                .await
            }
            _ => Scene::unsupported(args.as_str()),
        };

        cmd.processed = true;
        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    pub async fn add_entity(
        &mut self,
        args: &str,
        elp: EventLoopProxy<CommandEvent>,
    ) -> Option<Task<Vec<CommandEvent>>> {
        let vec_args: Vec<&str> = args.split("--").collect();

        let components: Vec<(&str, Vec<&str>)> = vec_args
            .iter()
            .map(|&arg| {
                let split: Vec<&str> = arg.split(' ').collect();
                (split[0], split[1..].to_vec())
            })
            .filter(|(name, args)| !name.is_empty() && !args.is_empty())
            .collect();

        let mut entity = self.world.spawn_empty();

        for (component_name, args) in components {
            match component_name {
                "name" => {
                    let name = name_component::NameComponent::from_args(args);
                    entity.insert(name);
                }
                "primitive" => {
                    let primitive = Primitive::from_args(args, elp.clone());
                    if let Some(primitive) = primitive {
                        entity.insert(primitive);
                    }
                }
                _ => {
                    error!(
                        "Unknown component: {} with args: {:?}",
                        component_name, args
                    );
                }
            }
        }
        None
    }

    pub fn remove_entity() {}

    pub fn cleanup() {}
}

#[async_trait(?Send)]
impl App for Scene {
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

        let load_test_tex = Command::new(
            "asset_server",
            CommandType::Get,
            Some("get textures/happy-tree.png texture".into()),
            None,
        );

        self.commands.append(&mut vec![
            load_basic_shader,
            load_line_shader,
            load_test_tex,
        ]);
    }

    async fn process_command(&mut self, cmd: Command) {
        self.process_scene_commands(cmd).await;
    }

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        self.commands.drain(..).collect()
    }

    async fn process_event(
        &mut self,
        event: &winit::event::Event<crate::core::events::CommandEvent>,
    ) {
        #[allow(clippy::single_match)]
        match event {
            winit::event::Event::WindowEvent {
                window_id,
                event: winit::event::WindowEvent::RedrawRequested,
            } => {
                let mut primitives_from_scene = self.world.query::<&Primitive>();
                let mut primitives_for_renderer = vec![];

                for primitive in primitives_from_scene.iter(&self.world) {
                    primitives_for_renderer.push(primitive.clone());
                }

                let render_desc = RenderDesc {
                    primitives: primitives_for_renderer,
                    window_id: *window_id,
                };
                self.proxy
                    .as_ref()
                    .unwrap()
                    .send_event(CommandEvent::Render(render_desc))
                    .unwrap();
            }
            winit::event::Event::UserEvent(CommandEvent::Asset(asset)) => {
                if asset.asset_type == AssetType::Texture {
                    info!("Yuppee");
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

impl Drop for Scene {
    fn drop(&mut self) {}
}
