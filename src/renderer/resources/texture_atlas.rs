use std::collections::HashMap;

use cgmath::Vector2;

use super::{image::Image, rect::Rect};

#[derive(Debug, Clone)]

pub struct TextureAtlas {
    // The texture of the atlas
    pub image: Image,

    // Size in pixels
    pub size: Vector2<f32>,

    // The specific areas of the atlas where each texture can be found
    pub textures: Vec<Rect<u32>>,

    // Texture id mapped to the index within the rect vector for when created from separate textures
    pub texture_handles: Option<HashMap<u16, usize>>,
}

impl TextureAtlas {
    /// Create a new `TextureAtlas` that has a texture, but does not have
    /// any individual sprites specified
    pub fn new_empty(image: Image, dimensions: Vector2<f32>) -> Self {
        Self {
            image,
            size: dimensions,
            textures: Vec::new(),
            texture_handles: None,
        }
    }

    /// Generate a `TextureAtlas` by splitting a texture into a grid where each
    /// cell of the grid  of `tile_size` is one of the textures in the atlas
    pub fn from_grid(
        image: Image,
        tile_size: Vector2<f32>,
        columns: usize,
        rows: usize,
    ) -> TextureAtlas {
        Self::from_grid_with_padding(image, tile_size, columns, rows, Vector2::new(0f32, 0f32))
    }

    /// Generate a `TextureAtlas` by splitting a texture into a grid where each
    /// cell of the grid of `tile_size` is one of the textures in the atlas and is separated by
    /// some `padding` in the texture
    pub fn from_grid_with_padding(
        image: Image,
        tile_size: Vector2<f32>,
        columns: usize,
        rows: usize,
        padding: Vector2<f32>,
    ) -> TextureAtlas {
        let mut sprites = Vec::new();
        let mut x_padding = 0.0;
        let mut y_padding = 0.0;

        for y in 0..rows {
            if y > 0 {
                y_padding = padding.y;
            }
            for x in 0..columns {
                if x > 0 {
                    x_padding = padding.x;
                }

                let rect_min = Vector2::new(
                    ((tile_size.x + x_padding) * x as f32) as u32,
                    ((tile_size.y + y_padding) * y as f32) as u32,
                );

                sprites.push(Rect {
                    min: rect_min,
                    max: Vector2::new(
                        rect_min.x + tile_size.x as u32,
                        rect_min.y + tile_size.y as u32,
                    ),
                })
            }
        }

        TextureAtlas {
            size: Vector2::new(
                ((tile_size.x + x_padding) * columns as f32) - x_padding,
                ((tile_size.y + y_padding) * rows as f32) - y_padding,
            ),
            textures: sprites,
            image,
            texture_handles: None,
        }
    }

    /// Add a sprite to the list of textures in the `TextureAtlas`
    ///
    /// # Arguments
    ///
    /// * `rect` - The section of the atlas that contains the texture to be added,
    /// from the top-left corner of the texture to the bottom-right corner
    pub fn add_texture(&mut self, rect: Rect<u32>) {
        self.textures.push(rect);
    }

    /// How many textures are in the `TextureAtlas`
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}
