use async_trait::async_trait;
use bevy_ecs::{entity::Entity, query::WorldQuery};
use tracing::{debug, error};
use winit::event_loop::EventLoopProxy;

use crate::{
    core::{
        app::App,
        command_queue::{Command, CommandType, Task},
        events::CommandEvent,
    },
    prelude::{
        name_component::NameComponent,
        primitive::Primitive,
        state::initialized,
        sun::{BufferDesc, RenderDesc},
        util,
    },
};

#[derive(Default)]
pub struct Scene {
    pub world: bevy_ecs::world::World,
    pub commands: Vec<Command>,

    pub temp: bool,

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
            "add" => self.add_entity(vec_args[1..].join(" ").as_str()).await,
            "remove" => self.remove_entity(vec_args[1..].join(" ")),
            _ => Scene::unsupported(args.as_str()),
        };

        cmd.processed = true;
        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    pub async fn add_entity(&mut self, args: &str) -> Option<Task<Vec<CommandEvent>>> {
        let components = util::extract_arguments(args);

        let mut render_data = Vec::new();

        let mut entity = self.world.spawn_empty();

        for (component_name, args) in components {
            match component_name {
                "name" => {
                    let name = NameComponent::from_args(args);
                    entity.insert(name);
                }
                "primitive" => {
                    let primitive = Primitive::from_args(args);
                    if let Some(primitive) = primitive {
                        render_data.push(primitive.clone());
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

        let task = move || {
            debug!("Add entity event!");
            let event = CommandEvent::RequestCreateBuffer(BufferDesc {
                data: render_data.clone(),
            });

            vec![event]
        };

        Some(Box::new(task))
    }

    pub fn query_world<Components: WorldQuery>(&mut self) -> Vec<Entity> {
        let mut query = self.world.query::<(Entity, Components)>();

        let mut entities: Vec<Entity> = Vec::new();

        for (entity, _comp) in query.iter(&self.world) {
            entities.push(entity);
        }

        entities
    }

    pub fn remove_entity(&mut self, name: String) -> Option<Task<Vec<CommandEvent>>> {
        let entitis = self.query_world::<&NameComponent>();

        let mut events = Vec::new();

        for entity in entitis {
            let mut flag = false;
            if self.world.get::<NameComponent>(entity).unwrap().name == name {
                if let Some(prim) = self.world.get::<Primitive>(entity) {
                    events.push(CommandEvent::RequestDestroyBuffer(prim.uuid));
                }
                flag = true;
            }

            if flag {
                self.world.despawn(entity);
            }
        }

        let task = move || {
            debug!("Add entity event!");

            events.clone()
        };

        Some(Box::new(task))
    }

    pub fn cleanup(&mut self) {
        let mut primitives_from_scene = self.world.query::<&Primitive>();

        for primitive in primitives_from_scene.iter(&self.world) {
            self.proxy
                .as_ref()
                .unwrap()
                .send_event(CommandEvent::RequestDestroyBuffer(primitive.uuid))
                .unwrap();
        }
    }
}

#[async_trait(?Send)]
impl App for Scene {
    fn init(&mut self, elp: EventLoopProxy<CommandEvent>) {
        self.proxy = Some(elp.clone());

        let load_test_tex = Command::new(
            "asset_server",
            CommandType::Get,
            Some("get textures/happy-tree.png texture".into()),
            None,
        );

        self.commands.append(&mut vec![load_test_tex]);
    }

    async fn process_command(&mut self, cmd: Command) {
        self.process_scene_commands(cmd).await;
    }

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        if initialized() {
            if !self.temp {
                self.temp = true;

                let load_basic_pentagon = Command::new(
                    "default_scene",
                    CommandType::Get,
                    Some("add --name Penta --primitive pentagon".into()),
                    None,
                );

                self.commands.push(load_basic_pentagon);
            }

            self.commands.drain(..).collect()
        } else {
            vec![]
        }
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
    fn drop(&mut self) {
        self.cleanup();
    }
}
