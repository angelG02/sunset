pub mod camera_component;
pub mod model_component;
pub mod name_component;
pub mod text_component;
pub mod transform_component;

#[derive(Debug, Clone)]
pub enum ChangeComponentState {
    Text(text_component::TextComponent),
    Transform(transform_component::TransformComponent),
    Model(model_component::ModelComponent),
}
