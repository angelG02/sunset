use bevy_ecs::component::Component;
use cgmath::{Vector2, Vector4, Zero};

use crate::prelude::{
    primitive::{Primitive, Quad2DVertex},
    resources::rect::Rect,
    sun::Viewport,
};

use super::{text_component::TextDesc, transform_component::TransformComponent};

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

/// A description that defines UI bounds, color and border
#[derive(Debug, Clone)]
pub struct ContainerDesc {
    // The window coordinates of the min and max points of the container
    pub bounds: Rect<f32>,
    // RGBA color with 0..1 range
    pub color: Vector4<f32>,
    // Border descritption
    pub border: BorderDesc,
    // Indicates changed state and to regen buffers
    pub changed: bool,
}

impl Default for ContainerDesc {
    fn default() -> Self {
        ContainerDesc {
            color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            changed: true,
            border: BorderDesc {
                width: 0.0,
                color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
            bounds: Rect {
                min: Vector2::zero(),
                max: Vector2::zero(),
            },
        }
    }
}

impl ContainerDesc {
    pub fn tesselate(
        &self,
        transform: &TransformComponent,
        vp: &Viewport, // TODO: This should be parent width and height
    ) -> (Vec<[Quad2DVertex; 4]>, [u32; 6]) {
        let mut quad_rect = self.bounds;
        quad_rect.min.y -= (self.bounds.max.y - self.bounds.min.y) + self.border.width;
        quad_rect.max.y -= (self.bounds.max.y - self.bounds.min.y) + self.border.width;

        quad_rect.min.x += self.border.width;
        quad_rect.max.x += self.border.width;

        // Border is technically a bigger rectangle under our container
        let border_rect = Rect {
            min: quad_rect.min - Vector2::new(self.border.width, self.border.width),
            max: quad_rect.max + Vector2::new(self.border.width, self.border.width),
        };

        // Pixel space to normalized device coordinates:
        // ndc_x = ((pixel_x / screen_width) * 2) - 1
        // ndc_y = ((pixel_y / screen_height) * 2) + 1 -> We want top left to be 0,0
        // (ndc rectangle (scaled) + ndc translation) - 1
        let ndc_rect = Rect {
            min: Vector2 {
                x: ((((quad_rect.min.x * transform.scale.x / vp.config.width as f32) * 2.0)
                    + ((transform.translation.x / vp.config.width as f32) * 2.0))
                    - 1.0),
                y: ((((quad_rect.min.y * transform.scale.y / vp.config.height as f32) * 2.0)
                    - (((transform.translation.y) / vp.config.height as f32) * 2.0)
                    + 2.0)
                    - 1.0),
            },
            max: Vector2 {
                x: ((((quad_rect.max.x * transform.scale.x / vp.config.width as f32) * 2.0)
                    + ((transform.translation.x / vp.config.width as f32) * 2.0))
                    - 1.0),
                y: ((((quad_rect.max.y * transform.scale.y / vp.config.height as f32) * 2.0)
                    - ((transform.translation.y / vp.config.height as f32) * 2.0)
                    + 2.0)
                    - 1.0),
            },
        };

        let ndc_rect_border = Rect {
            min: Vector2 {
                x: ((((border_rect.min.x * transform.scale.x / vp.config.width as f32) * 2.0)
                    + ((transform.translation.x / vp.config.width as f32) * 2.0))
                    - 1.0),
                y: ((((border_rect.min.y * transform.scale.y / vp.config.height as f32) * 2.0)
                    - ((transform.translation.y / vp.config.height as f32) * 2.0)
                    + 2.0)
                    - 1.0),
            },
            max: Vector2 {
                x: ((((border_rect.max.x * transform.scale.x / vp.config.width as f32) * 2.0)
                    + ((transform.translation.x / vp.config.width as f32) * 2.0))
                    - 1.0),
                y: ((((border_rect.max.y * transform.scale.y / vp.config.height as f32) * 2.0)
                    - ((transform.translation.y / vp.config.height as f32) * 2.0)
                    + 2.0)
                    - 1.0),
            },
        };

        let uvs = Rect {
            min: [0.0, 0.0].into(),
            max: [1.0, 1.0].into(),
        };

        let quad = Primitive::new_quad(ndc_rect, uvs, self.color, transform.translation.z as u16);
        let quad_border = Primitive::new_quad(
            ndc_rect_border,
            uvs,
            self.border.color,
            transform.translation.z as u16 - 1,
        );

        let (vert, _) = quad.data();
        let (vert_border, ind) = quad_border.data();

        (vec![vert_border, vert], ind)
    }
}

/// UI Component
///
/// Can be of type `UIType::Container(description)` or `UIType::Text(text_component)`
#[derive(Debug, Component, Clone)]
pub struct UIComponent {
    // Unique identifier (used for buffer generation)
    pub id: uuid::Uuid,
    pub string_id: String,
    pub ui_type: UIType,
}
