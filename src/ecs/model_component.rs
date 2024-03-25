use bevy_ecs::component::Component;

#[derive(Debug, Clone, Component)]
pub struct ModelComponent {
    pub id: uuid::Uuid,
    pub model_path: String,
}

impl ModelComponent {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            model_path: "".to_string(),
        }
    }

    pub fn from_args(args: Vec<&str>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            model_path: args[0].to_owned(),
        }
    }
}
