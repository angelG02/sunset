use bevy_ecs::component::Component;
use cgmath::Vector4;

use crate::prelude::resources::rect::Rect;

use super::text_component::TextComponent;

/// Deines what type the UI is
#[derive(Debug, Clone)]
pub enum UIType {
    Container(ContainerDesc),
    Text(TextComponent),
}

/// A description that defines the properties of a border around a UI Container
#[derive(Debug, Clone)]
pub struct BorderDesc {
    // Pixel width of the border
    pub width: f32,
    // RGBA color with 0..1 range
    pub color: Vector4<f32>,
}

impl Default for BorderDesc {
    fn default() -> Self {
        BorderDesc {
            width: 0.0,
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

/// A description that defines UI bounds, color and border
#[derive(Debug, Clone)]
pub struct ContainerDesc {
    // The window coordinates of the min and max points of the container
    pub bounds: Rect<f32>,
    // RGBA color with 0..1 range
    pub color: Vector4<f32>,
    // Border descritption
    pub border: BorderDesc,
}

impl Default for ContainerDesc {
    #[allow(unconditional_recursion)]
    fn default() -> Self {
        ContainerDesc {
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            ..Default::default()
        }
    }
}

/// UI Component
///
/// Can be of type `UIType::Container(description)` or `UIType::Text(text_component)`
#[derive(Debug, Component, Clone)]
pub struct UIComponent {
    pub id: String,
    pub ui_type: UIType,
}
