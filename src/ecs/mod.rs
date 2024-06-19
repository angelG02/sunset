use crate::prelude::resources::font::SunFont;

pub mod camera_component;
pub mod model_component;
pub mod name_component;
pub mod text_component;
pub mod transform_component;
pub mod ui_component;
pub mod window_component;

#[derive(Debug, Clone)]
pub enum ChangeComponentState {
    UI(
        (
            ui_component::UIComponent,
            Option<transform_component::TransformComponent>,
        ),
    ),
    Window(window_component::WindowContainer),
    FontAtlas(SunFont),
}
