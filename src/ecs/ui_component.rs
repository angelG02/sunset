use bevy_ecs::component::Component;
use cgmath::Vector4;

use crate::prelude::primitive::Quad2DVertex;

use super::text_component::TextDesc;

#[derive(Debug, Clone)]
pub struct UIQuadData {
    pub id: uuid::Uuid,
    pub vertices: Vec<([Quad2DVertex; 4], u16)>,
    pub changed: bool,
    pub ui_type: UIType,
}

// Data sent from Scene to Renderer to render text
#[derive(Debug, Clone)]
pub struct RenderUIDesc {
    pub geometry: (Vec<UIQuadData>, [u32; 6]),
}

/// Deines what type the UI is
#[derive(Debug, Clone)]
pub enum UIType {
    Container(ContainerDesc),
    Text(TextDesc),
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

/// Describes a screen coordinate value
#[derive(Debug, Clone)]
pub enum ScreenCoordinate {
    /// Define the value in pixels
    Pixels(f32),
    /// Defined in percentages of the parent (0..100)
    Percentage(u8),
}

/// A description that defines UI bounds, color and border
#[derive(Debug, Clone)]
pub struct ContainerDesc {
    // The width of the container (either in pixels or percentages)
    pub width: ScreenCoordinate,
    // The height of the container (either in pixels or percentages)
    pub height: ScreenCoordinate,
    // RGBA color with 0..1 range
    pub color: Vector4<f32>,
    // Border descritption
    pub border: BorderDesc,
    // Indicates changed state and to regen buffers
    pub changed: bool,
    pub focused_color: Option<Vector4<f32>>,
    pub focused: bool,
}

impl Default for ContainerDesc {
    fn default() -> Self {
        ContainerDesc {
            color: Vector4::new(0.0, 0.0, 0.0, 0.0),
            changed: true,
            border: BorderDesc {
                width: 0.0,
                color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
            width: ScreenCoordinate::Percentage(0),
            height: ScreenCoordinate::Percentage(0),
            focused_color: None,
            focused: false,
        }
    }
}

/// UI Component
///
/// Can be of type `UIType::Container(description)` or `UIType::Text(text_component)`
#[derive(Debug, Component, Clone)]
pub struct UIComponent {
    /// Unique identifier (used for buffer generation)
    pub id: uuid::Uuid,
    /// Unique string identifier (used for ease of access by other systems)
    pub string_id: String,
    /// Optional string id of parent container
    pub parent_id: Option<String>,
    /// UI Data defined by its type
    pub ui_type: UIType,
    /// Should the UI element be sent to the renderer for redraw
    pub visible: bool,
}
