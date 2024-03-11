use bevy_ecs::component::Component;

#[derive(Debug, Clone, Component)]
pub struct ModelComponent {
    pub id: uuid::Uuid,
    pub model_path: Option<String>,
    pub meshes: Vec<uuid::Uuid>,
    pub materials: Vec<uuid::Uuid>,
}

impl ModelComponent {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            model_path: None,
            meshes: vec![],
            materials: vec![],
        }
    }

    pub fn from_args(args: Vec<&str>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            model_path: Some(args[0].to_owned()),
            meshes: vec![],
            materials: vec![],
        }
    }
}
