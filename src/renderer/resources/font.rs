use std::io::Cursor;

use cgmath::Vector2;
use fdsm::{shape::Shape, transform::Transform};
use image::{GenericImage, Pixel, Rgba};

use crate::prelude::resources::image::{
    CompressedImageFormats, Image, ImageFormat, ImageSampler, ImageType,
};

use super::{texture_atlas::TextureAtlas, texture_atlas_builder::TextureAtlasBuilder};

pub struct SunFont {
    pub font_file: String,
    pub atlas: TextureAtlas,
}

impl SunFont {
    pub fn from_font_bytes(font_file: &str, data: &[u8]) -> anyhow::Result<Self> {
        // TODO (A40): Support Collections
        // let Some(index) = ttf_parser::fonts_in_collection(data) else {
        //     return Err(anyhow::Error::msg(format!(
        //         "Expected a True Type Font Collection. FILE: {font_file}"
        //     )));
        // };

        let mut texture_atlas_builder = TextureAtlasBuilder {
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            max_size: Vector2::new(4096, 4096),
            ..Default::default()
        };

        let font_face = ttf_parser::Face::parse(data, 0)?;
        let mut glyphs = Vec::new();

        // Enumerate all tables within the font face
        for table in font_face.tables().cmap.unwrap().subtables {
            let mut codes = Vec::new();
            // Get all the charcter codes within a table
            table.codepoints(|code| {
                codes.push(code);
            });

            // For each character code get its glyph id
            for code in codes {
                if let Some(glyph) = table.glyph_index(code) {
                    glyphs.push(glyph);
                }
            }
        }

        for glyph_id in glyphs {
            let mut shape = Shape::load_from_face(&font_face, glyph_id);

            // Prepare your transformation matrix and calculate the dimensions of
            // the resulting signed distance field. As an example, we set this up
            // using ‘shrinkage’ (font units per texel) and ‘range’ (number of
            // texels for the margin) values.

            // Note that since font files interpret a positive y-offset as
            // pointing up, the resulting distance field will be upside-down.
            // This can be corrected either by flipping the resulting image
            // vertically or by modifying the transformation matrix. We omit
            // this fix for simplicity.

            if let Some(bbox) = font_face.glyph_bounding_box(glyph_id) {
                const RANGE: f64 = 4.0;
                const SHRINKAGE: f64 = 16.0;

                let transformation =
                    nalgebra::convert::<_, nalgebra::Affine2<f64>>(nalgebra::Similarity2::new(
                        nalgebra::Vector2::new(
                            RANGE - bbox.x_min as f64 / SHRINKAGE,
                            RANGE - bbox.y_min as f64 / SHRINKAGE,
                        ),
                        0.0,
                        1.0 / SHRINKAGE,
                    ));
                let width = ((bbox.x_max as f64 - bbox.x_min as f64) / SHRINKAGE + 2.0 * RANGE)
                    .ceil() as u32;
                let height = ((bbox.y_max as f64 - bbox.y_min as f64) / SHRINKAGE + 2.0 * RANGE)
                    .ceil() as u32;

                // Unlike msdfgen, the transformation is not passed into the
                // `generate_msdf` function – the coordinates of the control points
                // must be expressed in terms of pixels on the distance field. To get
                // the correct units, we pre-transform the shape:

                shape.transform(&transformation);
                // We now color the edges of the shape. We also have to prepare
                // it for calculations:

                let colored_shape = Shape::edge_coloring_simple(shape, 0.03, 69441337420).prepare();

                // Set up the resulting image and generate the distance field:

                let mut msdf = image::RgbImage::new(width, height);
                fdsm::generate::generate_msdf(&colored_shape, RANGE, &mut msdf);
                image::imageops::flip_vertical_in_place(&mut msdf);

                let dynamic_image = image::DynamicImage::ImageRgb8(msdf);

                let image = Image::from_dynamic(dynamic_image, true);

                texture_atlas_builder.add_texture(Some(glyph_id.0), image);
            }
        }

        let atlas = texture_atlas_builder.finish()?;

        let buf = image::RgbaImage::from_raw(
            atlas.image.width(),
            atlas.image.height(),
            atlas.image.data.clone(),
        )
        .unwrap();
        buf.save("out.png")?;

        Ok(Self {
            font_file: font_file.to_owned(),
            atlas,
        })
    }
}
