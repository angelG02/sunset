use bevy_ecs::component::Component;
use cgmath::Vector4;

#[derive(Debug, Clone, Component)]
pub struct TextComponent {
    // The text to display
    pub text: String,
    // Handle to the font file
    pub font: String,
    // Color of the text (0..1)
    pub color: Vector4<f32>,
    // Space in between lines (in world units)
    pub line_spacing: f32,
    // Space in between individual characters (in world units)
    pub kerning: f32,
    // When to break the text into the next line
    pub max_width: f32,
}

impl Default for TextComponent {
    fn default() -> Self {
        Self {
            text: Default::default(),
            font: "OpenSans-Regular.ttf".to_string(),
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            kerning: Default::default(),
            line_spacing: Default::default(),
            max_width: Default::default(),
        }
    }
}
