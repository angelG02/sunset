use bevy_ecs::component::Component;

#[derive(Debug, Clone, Default, Component)]
pub struct WindowContainer {
    pub width: f32,
    pub height: f32,
}
