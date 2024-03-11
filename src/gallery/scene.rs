use async_trait::async_trait;
use bevy_ecs::{
    entity::Entity,
    query::{QueryFilter, With},
};
use tracing::{error, warn};
use winit::event_loop::EventLoopProxy;

use crate::{
    core::{
        app::App,
        command_queue::{Command, CommandType, Task},
        events::CommandEvent,
        state::initialized,
        util,
    },
    ecs::{
        camera_component::{ActiveCameraComponent, CameraComponent},
        model_component::ModelComponent,
        name_component::NameComponent,
    },
    renderer::{primitive::Primitive, sun::RenderDesc},
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
            "add" => self.add_entity(vec_args[1..].join(" ").as_str()).await,
            "remove" => self.remove_entity(vec_args[1..].join(" ").as_str()),
            "set_texture" => {
                if vec_args[1..].len() >= 2 {
                    self.set_texture(vec_args[1], vec_args[2])
                } else {
                    error!("Expected 2 or more arguments");
                    None
                }
            }
            _ => Scene::unsupported(args.as_str()),
        };

        cmd.processed = true;
        cmd.task = task;
        cmd.args = Some(args);

        self.commands.push(cmd);
    }

    pub async fn add_entity(&mut self, args: &str) -> Option<Task<Vec<CommandEvent>>> {
        let components = util::extract_arguments(args);

        let mut events = Vec::new();

        let mut entity = self.world.spawn_empty();

        for (component_name, args) in components {
            match component_name {
                "name" => {
                    let name = NameComponent::from_args(args).unwrap();
                    entity.insert(name);
                }
                "model" => {
                    let model = ModelComponent::from_args(args.clone());
                    events.push(CommandEvent::RequestCreateModel(model.clone()));
                    entity.insert(model);
                }
                "camera" => {
                    let camera = CameraComponent::from_args(args.clone());
                    if let Some(cam) = camera {
                        entity.insert(cam);

                        warn!("Note: (@A40) Please change active camera functionality!");
                        entity.insert(ActiveCameraComponent {});
                    } else {
                        error!(
                            "Failed to create component <{}> with args <{:?}>",
                            component_name,
                            args.clone()
                        );
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

        let task = move || events.clone();

        Some(Box::new(task))
    }

    pub fn get_entity_with_name(&mut self, name: &str) -> Option<Entity> {
        let entities_with_name = self.query_world::<With<NameComponent>>();

        for entity in entities_with_name {
            let name_cmp = self.world.get::<NameComponent>(entity).unwrap();
            if name == name_cmp.name {
                return Some(entity);
            }
        }
        None
    }

    pub fn query_world<Filter: QueryFilter>(&mut self) -> Vec<Entity> {
        let mut query = self.world.query_filtered::<Entity, Filter>();

        let entities: Vec<Entity> = query.iter(&self.world).collect();

        entities
    }

    pub fn set_active_camera(&mut self, new_active_camera: Entity) {
        let cam_entities =
            self.query_world::<(With<CameraComponent>, With<ActiveCameraComponent>)>();

        for e in cam_entities {
            self.world
                .get_entity_mut(e)
                .unwrap()
                .remove::<ActiveCameraComponent>();
        }

        self.world
            .get_entity_mut(new_active_camera)
            .unwrap()
            .insert(ActiveCameraComponent {});
    }

    pub fn set_texture(
        &mut self,
        tex_name: &str,
        entity_name: &str,
    ) -> Option<Task<Vec<CommandEvent>>> {
        let entity = self.get_entity_with_name(entity_name);
        if let Some(e) = entity {
            let primitive = self.world.get_mut::<Primitive>(e);
            if let Some(mut p) = primitive {
                p.temp_diffuse = Some(tex_name.to_owned());
            } else {
                error!("Entity <{}> has no primitive component!", entity_name);
            }
        } else {
            error!("Entity <{}> not found!", entity_name);
        }

        None
    }

    pub fn remove_entity(&mut self, name: &str) -> Option<Task<Vec<CommandEvent>>> {
        let entitis = self.query_world::<With<NameComponent>>();

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

        let task = move || events.clone();

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

        let load_missing_tex = Command::new(
            "asset_server",
            CommandType::Get,
            Some("get textures/missing.jpg texture".into()),
            None,
        );

        let load_basic_cube = Command::new(
            "default_scene",
            CommandType::Get,
            Some("add --name Cube --model models/avocado/avocado.glb".into()),
            None,
        );

        let load_camera_2d = Command::new(
            "default_scene",
            CommandType::Get,
            Some("add --name Camera2D --camera 2D -1 1 -1 1 0 0 1 0 1".into()),
            None,
        );

        self.commands
            .append(&mut vec![load_missing_tex, load_basic_cube, load_camera_2d]);
    }

    async fn process_command(&mut self, cmd: Command) {
        self.process_scene_commands(cmd).await;
    }

    fn update(&mut self /*schedule: Schedule, */) -> Vec<Command> {
        if initialized() {
            self.commands.drain(..).collect()
        } else {
            vec![]
        }
    }

    async fn process_window_event(
        &mut self,
        event: &winit::event::WindowEvent,
        window_id: winit::window::WindowId,
    ) {
        #[allow(clippy::single_match)]
        match event {
            winit::event::WindowEvent::RedrawRequested => {
                let mut primitives_from_scene = self.world.query::<&Primitive>();
                let mut primitives_for_renderer = vec![];

                for primitive in primitives_from_scene.iter(&self.world) {
                    primitives_for_renderer.push(primitive.clone());
                }

                let active_cams = self.query_world::<With<ActiveCameraComponent>>();

                let active_cam: CameraComponent = if active_cams.len() > 0 {
                    self.world
                        .get_entity(active_cams[0])
                        .unwrap()
                        .get::<CameraComponent>()
                        .unwrap()
                        .clone()
                } else {
                    CameraComponent::default()
                };

                let render_desc = RenderDesc {
                    primitives: primitives_for_renderer,
                    active_camera: active_cam,
                    window_id,
                };
                self.proxy
                    .as_ref()
                    .unwrap()
                    .send_event(CommandEvent::Render(render_desc))
                    .unwrap();
            }
            // winit::event::Event::WindowEvent {
            //     window_id: _,
            //     event:
            //         winit::event::WindowEvent::KeyboardInput {
            //             device_id: _,
            //             event,
            //             is_synthetic: _,
            //         },
            // } => {
            //     if event.physical_key
            //         == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F1)
            //         && event.state == winit::event::ElementState::Released
            //     {

            //     }
            // }
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
