use cgmath::Vector4;

#[derive(Debug, Clone)]
pub struct TextDesc {
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
    // Has the text changed since last buffer update
    pub changed: bool,
}

impl Default for TextDesc {
    fn default() -> Self {
        Self {
            text: "TextDesc".to_string(),
            font: "OpenSans-Regular.ttf".to_string(),
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            kerning: 0.0,
            line_spacing: 1.0,
            max_width: f32::MAX,
            changed: true,
        }
    }
}
