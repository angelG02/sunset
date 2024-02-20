use bevy_ecs::component::Component;

#[derive(Debug, Component)]
pub struct NameComponent {
    pub name: String,
}

impl NameComponent {
    pub fn from_args(args: Vec<&str>) -> Option<Self> {
        Some(Self {
            name: args[0].to_owned(),
        })
    }
}
