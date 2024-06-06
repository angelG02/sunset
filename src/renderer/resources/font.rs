use std::sync::Arc;

use async_std::channel::{Sender, TryRecvError};
use cgmath::Vector2;
use fdsm::{
    bezier::Segment,
    shape::{Shape, TContour},
    transform::Transform,
};
use tracing::{error, info};
use ttf_parser::{GlyphId, Rect};

use crate::prelude::resources::image::Image;

use super::{texture_atlas::TextureAtlas, texture_atlas_builder::TextureAtlasBuilder};

pub struct SunFont {
    pub font_file: String,
    pub atlas: TextureAtlas,
}

impl SunFont {
    pub async fn from_font_bytes(font_file: &str, data: &[u8]) -> anyhow::Result<Self> {
        let time = web_time::Instant::now();
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

        let (image_sender, image_receiver) = async_std::channel::unbounded::<(u16, Arc<Image>)>();

        // We do not use tokio multi-threaded executor on the web (we use the browser's own executor for js promises)
        #[cfg(not(target_arch = "wasm32"))]
        use tokio::task::JoinSet;

        #[cfg(not(target_arch = "wasm32"))]
        let mut join_set = JoinSet::new();

        let mut glyph_index = 0;
        let expected_glyphs = glyphs.len() as u32;

        info!("{expected_glyphs}");

        // For each glyph clone the shape dataand create a task that can be Sent between threads
        // clone an image sender to send an image whenever the task finishes
        for glyph_id in glyphs {
            let shape = Shape::load_from_face(&font_face, glyph_id);
            if let Some(bbox) = font_face.glyph_bounding_box(glyph_id) {
                let task = SunFont::gen_glyph_image(
                    shape.clone(),
                    bbox,
                    glyph_id,
                    image_sender.clone(),
                    glyph_index,
                    expected_glyphs,
                );

                #[cfg(target_arch = "wasm32")]
                task.await;

                #[cfg(not(target_arch = "wasm32"))]
                join_set.spawn(task);
            }
            glyph_index += 1;
        }

        // We execute our tasks in parallel filling up the buffer of the image receiver
        #[cfg(not(target_arch = "wasm32"))]
        while let Some(_) = join_set.join_next().await {
            //info!("Waiting for image gen");
        }

        // Main thread!
        // Loop until we recieve all images from the senders dispthched (indicated by all depleting the buffer of images)
        loop {
            let res = image_receiver.try_recv();

            match res {
                Ok((id, image)) => texture_atlas_builder.add_texture(Some(id), image),
                Err(err) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    if err == TryRecvError::Empty {
                        break;
                    }
                    #[cfg(target_arch = "wasm32")]
                    if err == TryRecvError::Closed {
                        break;
                    }
                }
            }
        }

        let atlas = texture_atlas_builder.finish()?;

        let time_elapsed = time.elapsed().as_millis();
        info!("It took {time_elapsed}ms to generate font");

        Ok(Self {
            font_file: font_file.to_owned(),
            atlas,
        })
    }

    async fn gen_glyph_image(
        mut shape: Shape<TContour<Segment>>,
        bbox: Rect,
        glyph_id: GlyphId,
        image_sender: Sender<(u16, Arc<Image>)>,
        glyph_index: u32,
        expected_glyphs: u32,
    ) {
        // Prepare your transformation matrix and calculate the dimensions of
        // the resulting signed distance field. As an example, we set this up
        // using ‘shrinkage’ (font units per texel) and ‘range’ (number of
        // texels for the margin) values.

        // Note that since font files interpret a positive y-offset as
        // pointing up, the resulting distance field will be upside-down.
        // This can be corrected either by flipping the resulting image
        // vertically or by modifying the transformation matrix. We omit
        // this fix for simplicity.

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
        let width =
            ((bbox.x_max as f64 - bbox.x_min as f64) / SHRINKAGE + 2.0 * RANGE).ceil() as u32;
        let height =
            ((bbox.y_max as f64 - bbox.y_min as f64) / SHRINKAGE + 2.0 * RANGE).ceil() as u32;

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

        image_sender
            .send((glyph_id.0, Arc::new(image)))
            .await
            .unwrap_or_else(|e| {
                error!("Failed to send image over channle! {e}");
                ()
            });

        if glyph_index >= expected_glyphs - 1 {
            image_sender.close();
        }
    }
}
