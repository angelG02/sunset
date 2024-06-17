use bevy_ecs::component::Component;
use cgmath::{Vector2, Vector4};

use crate::prelude::{
    primitive::{Primitive, Quad2DVertex},
    resources::{font::SunFont, rect::Rect},
    sun::Viewport,
};

use super::{transform_component::TransformComponent, ui_component::UIComponent};

// Data sent from Scene to Renderer to render text
#[derive(Debug, Clone)]
pub struct RenderUIDesc {
    pub uis: Vec<(UIComponent, TransformComponent)>,
}

#[derive(Debug, Clone, Component)]
pub struct TextComponent {
    // Unique identifier (used for buffer generation)
    pub id: uuid::Uuid,
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

impl Default for TextComponent {
    fn default() -> Self {
        Self {
            text: "TextComponent".to_string(),
            font: "OpenSans-Regular.ttf".to_string(),
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            kerning: 0.0,
            line_spacing: 1.0,
            max_width: f32::MAX,
            changed: true,
            id: uuid::Uuid::new_v4(),
        }
    }
}

impl TextComponent {
    // Based on a provided transform, font and viewport,
    // calculate the vertices, indices and texture coordiantes of each character in the text component's text
    pub fn tesselate(
        &self,
        font: &SunFont,
        transform: &TransformComponent,
        vp: &Viewport,
    ) -> (Vec<[Quad2DVertex; 4]>, [u32; 6]) {
        let mut vertices = vec![];
        let mut indices = [0u32, 0, 0, 0, 0, 0];

        // Unwrap safety: in order to generate the atlas the data has already been parsed successfully
        let font_face = ttf_parser::Face::parse(&font.font_data, 0).unwrap();

        let question_mark_id = font_face
            .glyph_index('?')
            .expect("How TF can this font not have '?'");

        let characters = self.text.chars();
        let characters_count = self.text.len();

        // X coordinate
        let mut x = 0f32;

        // Font size scale
        let fs_scale = 1.0 / (font_face.ascender() as f32 - font_face.descender() as f32);

        // Y coordinate
        let mut y = 0f32;

        let space_glyph_advance = font_face
            .glyph_hor_advance(
                font_face
                    .glyph_index(' ')
                    .expect("HOW TF CAN A FONT NOT HAVE THE SPACE CHAR"),
            )
            .unwrap();

        for (index, character) in characters.enumerate() {
            if character == '\r' {
                continue;
            }

            // Reset x and increase y by the appropriate amount
            if character == '\n' {
                x = 0f32;
                y -= fs_scale * font_face.line_gap() as f32 + self.line_spacing;
                continue;
            }

            // Check to see if there are any more character and get the spacing in between to advance the x by
            if character == ' ' {
                let mut advance = space_glyph_advance;

                if index < characters_count - 1 {
                    let next_char = self.text.as_bytes()[index + 1] as char;
                    let next_char_id = font_face.glyph_index(next_char).unwrap_or(question_mark_id);
                    advance = font_face.glyph_hor_advance(next_char_id).unwrap();
                }

                x += fs_scale * advance as f32 + self.kerning;
                continue;
            }

            // Add four spaces
            if character == '\t' {
                x += 4.0 * (fs_scale * space_glyph_advance as f32 + self.kerning);
                continue;
            }

            let glyph_id = font_face.glyph_index(character).unwrap_or(question_mark_id);

            // Unwrap safety: a FONT atlas will always have handle to the individual characters
            let font_handles = font.atlas.texture_handles.as_ref().unwrap();
            let font_textures = &font.atlas.textures;

            // Unwrap safety: there is always a handle to match the glyph id for all glyphs within a font
            let handle = *font_handles.get(&glyph_id.0).unwrap();

            // The texture coords within the atlas are in pixel space
            let rect = font_textures[handle];

            // Convert to texture space by dividing by the width and height
            let uvs = Rect {
                min: Vector2 {
                    x: rect.min.x as f32 / font.atlas.size.x,
                    y: rect.min.y as f32 / font.atlas.size.y,
                },
                max: Vector2 {
                    x: rect.max.x as f32 / font.atlas.size.x,
                    y: rect.max.y as f32 / font.atlas.size.y,
                },
            };

            let quad_plane_bounds = font_face.glyph_bounding_box(glyph_id).unwrap();

            // Creare a bounding box out of the glyph
            let mut quad_rect = Rect {
                min: Vector2::new(
                    quad_plane_bounds.x_min as f32,
                    quad_plane_bounds.y_min as f32,
                ),
                max: Vector2::new(
                    quad_plane_bounds.x_max as f32,
                    quad_plane_bounds.y_max as f32,
                ),
            };

            // Scale the bounding box
            quad_rect.min *= fs_scale;
            quad_rect.max *= fs_scale;

            // Position it accordingly
            quad_rect.min += Vector2::new(x, y);
            quad_rect.max += Vector2::new(x, y);

            // Pixel space to normalized device coordinates:
            // ndc_x = ((pixel_x / screen_width) * 2) - 1
            // (ndc rectangle (scaled) + ndc translation) - 1
            let ndc_rect = Rect {
                min: Vector2 {
                    x: ((((quad_rect.min.x * transform.scale.x / vp.config.width as f32) * 2.0)
                        + ((transform.translation.x / vp.config.width as f32) * 2.0))
                        - 1.0),
                    y: ((((quad_rect.min.y * transform.scale.y / vp.config.height as f32) * 2.0)
                        + ((transform.translation.y / vp.config.height as f32) * 2.0))
                        - 1.0),
                },
                max: Vector2 {
                    x: ((((quad_rect.max.x * transform.scale.x / vp.config.width as f32) * 2.0)
                        + ((transform.translation.x / vp.config.width as f32) * 2.0))
                        - 1.0),
                    y: ((((quad_rect.max.y * transform.scale.y / vp.config.height as f32) * 2.0)
                        + ((transform.translation.y / vp.config.height as f32) * 2.0))
                        - 1.0),
                },
            };

            let quad = Primitive::new_quad(ndc_rect, uvs, self.color);

            let (vert, ind) = match quad {
                Primitive::Quad(data) => (data.vertices, data.indices),
                _ => unreachable!(),
            };

            vertices.push(vert);
            indices = ind;

            // After each letter add the needed space for the next
            if index < characters_count - 1 {
                let advance = font_face.glyph_hor_advance(glyph_id).unwrap();
                x += fs_scale * advance as f32 + self.kerning;
            }
        }

        (vertices, indices)
    }
}
