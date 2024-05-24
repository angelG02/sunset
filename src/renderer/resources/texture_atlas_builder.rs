use std::collections::HashMap;

use cgmath::{Vector2, Zero};
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, TargetBin,
};
use thiserror::Error;
use tracing::{error, warn};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use super::{
    image::{Image, TextureFormatPixelInfo},
    rect::Rect,
    texture_atlas::TextureAtlas,
};

#[derive(Debug, Error)]
pub enum TextureAtlasBuilderError {
    #[error("could not pack textures into an atlas within the given bounds")]
    NotEnoughSpace,
    #[error("added a texture with the wrong format in an atlas")]
    WrongFormat,
}

#[derive(Debug)]
#[must_use]
/// A builder which is used to create a texture atlas from many individual
/// sprites.
pub struct TextureAtlasBuilder {
    /// Collection of texture's asset id (optional) and image data to be packed into an atlas
    pub textures_to_place: Vec<(Option<u16>, Image)>,
    /// The initial atlas size in pixels.
    pub initial_size: Vector2<u32>,
    /// The absolute maximum size of the texture atlas in pixels.
    pub max_size: Vector2<u32>,
    /// The texture format for the textures that will be loaded in the atlas.
    pub format: TextureFormat,
    /// Enable automatic format conversion for textures if they are not in the atlas format.
    pub auto_format_conversion: bool,
    /// The amount of padding in pixels to add along the right and bottom edges of the texture rects.
    pub padding: Vector2<u32>,
}

impl Default for TextureAtlasBuilder {
    fn default() -> Self {
        Self {
            textures_to_place: Vec::new(),
            initial_size: Vector2::<u32>::new(256, 256),
            max_size: Vector2::<u32>::new(2048, 2048),
            format: TextureFormat::Rgba8UnormSrgb,
            auto_format_conversion: true,
            padding: Vector2::<u32>::zero(),
        }
    }
}

pub type TextureAtlasBuilderResult<T> = Result<T, TextureAtlasBuilderError>;

impl TextureAtlasBuilder {
    /// Sets the initial size of the atlas in pixels.
    pub fn initial_size(mut self, size: Vector2<u32>) -> Self {
        self.initial_size = size;
        self
    }

    /// Sets the max size of the atlas in pixels.
    pub fn max_size(mut self, size: Vector2<u32>) -> Self {
        self.max_size = size;
        self
    }

    /// Sets the texture format for textures in the atlas.
    pub fn format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Control whether the added texture should be converted to the atlas format, if different.
    pub fn auto_format_conversion(mut self, auto_format_conversion: bool) -> Self {
        self.auto_format_conversion = auto_format_conversion;
        self
    }

    /// Adds a texture to be copied to the texture atlas.
    ///
    /// Optionally an asset id can be passed that can later be used with the texture layout to retrieve the index of this texture.
    /// The insertion order will reflect the index of the added texture in the finished texture atlas.
    pub fn add_texture(&mut self, image_id: Option<u16>, texture: Image) {
        self.textures_to_place.push((image_id, texture));
    }

    /// Sets the amount of padding in pixels to add between the textures in the texture atlas.
    ///
    /// The `x` value provide will be added to the right edge, while the `y` value will be added to the bottom edge.
    pub fn padding(mut self, padding: Vector2<u32>) -> Self {
        self.padding = padding;
        self
    }

    fn copy_texture_to_atlas(
        atlas_texture: &mut Image,
        texture: &Image,
        packed_location: &PackedLocation,
        padding: Vector2<u32>,
    ) {
        let rect_width = (packed_location.width() - padding.x) as usize;
        let rect_height = (packed_location.height() - padding.y) as usize;
        let rect_x = packed_location.x() as usize;
        let rect_y = packed_location.y() as usize;
        let atlas_width = atlas_texture.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size();

        for (texture_y, bound_y) in (rect_y..rect_y + rect_height).enumerate() {
            let begin = (bound_y * atlas_width + rect_x) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_texture.data[begin..end]
                .copy_from_slice(&texture.data[texture_begin..texture_end]);
        }
    }

    fn copy_converted_texture(
        &self,
        atlas_texture: &mut Image,
        texture: &Image,
        packed_location: &PackedLocation,
    ) {
        if self.format == texture.texture_descriptor.format {
            Self::copy_texture_to_atlas(atlas_texture, texture, packed_location, self.padding);
        } else {
            error!(
                "Error converting texture from '{:?}' to '{:?}', ignoring",
                texture.texture_descriptor.format, self.format
            );
        }
    }

    /// Consumes the builder, and returns the newly created texture atlas and
    /// the associated atlas layout.
    ///
    /// Assigns indices to the textures based on the insertion order.
    /// Internally it copies all rectangles from the textures and copies them
    /// into a new texture.
    ///
    /// # Errors
    ///
    /// If there is not enough space in the atlas texture, an error will
    /// be returned. It is then recommended to make a larger sprite sheet.
    pub fn finish(self) -> Result<TextureAtlas, TextureAtlasBuilderError> {
        let max_width = self.max_size.x;
        let max_height = self.max_size.y;

        let mut current_width = self.initial_size.x;
        let mut current_height = self.initial_size.y;
        let mut rect_placements = None;
        let mut atlas_texture = Image::default();
        let mut rects_to_place = GroupedRectsToPlace::<usize>::new();

        // Adds textures to rectangle group packer
        for (index, (_, texture)) in self.textures_to_place.iter().enumerate() {
            rects_to_place.push_rect(
                index,
                None,
                RectToInsert::new(
                    texture.width() + self.padding.x,
                    texture.height() + self.padding.y,
                    1,
                ),
            );
        }

        while rect_placements.is_none() {
            if current_width > max_width || current_height > max_height {
                break;
            }

            let last_attempt = current_height == max_height && current_width == max_width;

            let mut target_bins = std::collections::BTreeMap::new();
            target_bins.insert(0, TargetBin::new(current_width, current_height, 1));
            rect_placements = match pack_rects(
                &rects_to_place,
                &mut target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(rect_placements) => {
                    atlas_texture = Image::new(
                        Extent3d {
                            width: current_width,
                            height: current_height,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        vec![
                            0;
                            self.format.pixel_size() * (current_width * current_height) as usize
                        ],
                        self.format,
                    );
                    Some(rect_placements)
                }
                Err(rectangle_pack::RectanglePackError::NotEnoughBinSpace) => {
                    current_height = (current_height * 2).clamp(0, max_height);
                    current_width = (current_width * 2).clamp(0, max_width);
                    None
                }
            };

            if last_attempt {
                break;
            }
        }

        let rect_placements = rect_placements.ok_or(TextureAtlasBuilderError::NotEnoughSpace)?;

        let mut texture_rects = Vec::with_capacity(rect_placements.packed_locations().len());
        let mut texture_ids = HashMap::default();
        // We iterate through the textures to place to respect the insertion order for the texture indices
        for (index, (image_id, texture)) in self.textures_to_place.iter().enumerate() {
            let (_, packed_location) = rect_placements.packed_locations().get(&index).unwrap();

            let min: Vector2<u32> = Vector2::new(packed_location.x(), packed_location.y());
            let max = min + Vector2::new(packed_location.width(), packed_location.height())
                - self.padding;
            if let Some(image_id) = image_id {
                texture_ids.insert(*image_id, index);
            }
            texture_rects.push(Rect { min, max });
            if texture.texture_descriptor.format != self.format && !self.auto_format_conversion {
                warn!(
                    "Loading a texture of format '{:?}' in an atlas with format '{:?}'",
                    texture.texture_descriptor.format, self.format
                );
                return Err(TextureAtlasBuilderError::WrongFormat);
            }
            self.copy_converted_texture(&mut atlas_texture, texture, packed_location);
        }

        Ok(TextureAtlas {
            size: atlas_texture.size(),
            image: atlas_texture,
            textures: texture_rects,
            texture_handles: Some(texture_ids),
        })
    }
}
