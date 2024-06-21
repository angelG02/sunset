use std::collections::HashMap;

use bevy_ecs::{entity::Entity, world::World};
use cgmath::{Vector2, Vector4};
use tracing::{error, info};

use crate::prelude::{
    primitive::Primitive,
    resources::{font::SunFont, rect::Rect},
    transform_component::TransformComponent,
    ui_component::{BorderDesc, ContainerDesc, ScreenCoordinate, UIComponent, UIQuadData, UIType},
    window_component::WindowContainer,
    ChangeComponentState,
};

#[derive(Default)]
pub struct UIHandler {
    pub id_map: HashMap<String, Entity>,
    pub window_container: WindowContainer,
    pub fonts: HashMap<String, SunFont>,
}

impl UIHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_handle(&mut self, ui_string_id: String, entity: Entity) {
        self.id_map.insert(ui_string_id, entity);
    }

    pub fn add_font(&mut self, font_name: String, font: SunFont) {
        self.fonts.insert(font_name, font);
    }

    pub fn init_ui(&mut self, world: &mut World) {
        // Left side-bar
        let mut quad_transform = TransformComponent::zero();
        quad_transform.translation.z = 0.0;

        let ui_quad = UIComponent {
            // Id uesd for creating & indexing into vertex/index buffers in renderer
            id: uuid::Uuid::new_v4(),
            // Id uesd for parenting and accessing through events
            string_id: "test".to_string(),
            parent_id: None,
            // Define our UI container
            ui_type: UIType::Container(ContainerDesc {
                // Set width to 25% of the parent
                width: ScreenCoordinate::Percentage(25),
                // Set height to 100% of the parent
                height: ScreenCoordinate::Percentage(100),
                // Set color
                color: Vector4::new(4.0 / 255.0, 229.0 / 255.0, 218.0 / 255.0, 1.0),
                // Set border color and width
                border: BorderDesc {
                    color: Vector4::new(4.0 / 255.0, 4.0 / 255.0, 229.0 / 255.0, 1.0),
                    width: 10.0,
                },
                ..Default::default()
            }),
            visible: true,
        };

        let mut e = world.spawn_empty();

        self.add_handle(ui_quad.string_id.clone(), e.id());

        e.insert((quad_transform, ui_quad));

        let mut quad_transform = TransformComponent::zero();
        quad_transform.translation.z = 1.0;
        quad_transform.translation.y += 150.0;

        let ui_quad = UIComponent {
            // Id uesd for creating & indexing into vertex/index buffers in renderer
            id: uuid::Uuid::new_v4(),
            // Id uesd for parenting and accessing through events
            string_id: "test-child".to_string(),
            parent_id: Some("test".into()),
            // Define our UI container
            ui_type: UIType::Container(ContainerDesc {
                // Set width to 100% of the parent
                width: ScreenCoordinate::Percentage(100),
                // Set height to 50% of the parent
                height: ScreenCoordinate::Percentage(50),
                // Set color
                color: Vector4::new(200.0 / 255.0, 5.0 / 255.0, 218.0 / 255.0, 1.0),
                border: BorderDesc {
                    color: Vector4::new(4.0 / 255.0, 4.0 / 255.0, 229.0 / 255.0, 1.0),
                    width: 10.0,
                },
                ..Default::default()
            }),
            visible: true,
        };

        let mut e = world.spawn_empty();

        self.add_handle(ui_quad.string_id.clone(), e.id());

        e.insert((quad_transform, ui_quad));
    }

    pub fn on_change_component_state(
        &mut self,
        change_state: &ChangeComponentState,
        world: &mut World,
    ) {
        match change_state {
            // Replace/Add Text
            crate::prelude::ChangeComponentState::UI((changed_ui, changed_transform)) => {
                let id = &changed_ui.string_id;
                if let Some(entity) = self.id_map.get(id) {
                    // Replace or spawn component of the requested type on the provided entity
                    if let Some(mut ui) = world.get_mut::<UIComponent>(*entity) {
                        *ui = UIComponent {
                            id: ui.id,
                            string_id: ui.string_id.clone(),
                            ui_type: changed_ui.ui_type.clone(),
                            visible: ui.visible,
                            parent_id: ui.parent_id.clone(),
                        };
                    } else {
                        world.entity_mut(*entity).insert(changed_ui.clone());
                    }

                    if changed_transform.is_some() {
                        if let Some(mut transform) = world.get_mut::<TransformComponent>(*entity) {
                            *transform = changed_transform.clone().unwrap();
                        } else {
                            world
                                .entity_mut(*entity)
                                .insert(changed_transform.clone().unwrap());
                        }
                    }
                } else {
                    let mut entity = world.spawn_empty();

                    entity.insert(changed_ui.clone());

                    if let Some(trans) = changed_transform {
                        entity.insert(trans.clone());
                    }
                    self.id_map.insert(id.clone(), entity.id());
                }
            }
            ChangeComponentState::Window(window) => {
                self.window_container = window.clone();
            }
            ChangeComponentState::FontAtlas(font) => {
                self.fonts.insert(font.font_file.clone(), font.clone());
            }
        }
    }

    /// TODO: Change check in tessellate
    ///
    /// Returns a tuple of Vec(UIQuadData), changed) and an index array
    pub fn tessellate(&self, world: &mut World) -> (Vec<UIQuadData>, [u32; 6]) {
        // Gether ui elements and their transforms from the scene
        let mut text_from_scene = world.query::<(&mut UIComponent, &TransformComponent)>();

        let indices = [0u32, 1u32, 2u32, 0u32, 2u32, 3u32];
        let mut ui_quad_data_vec = vec![];

        for (ui, transform) in text_from_scene.iter(&world) {
            // Parent width and height
            let mut parent_width = self.window_container.width;
            let mut parent_height = self.window_container.height;
            let mut parent_transform = TransformComponent::zero();

            let mut parent = ui.parent_id.clone();

            // Iteratively get to the top-most parent and get its width and height
            // or calculate what percentage of the element's parent they take
            loop {
                if parent.is_none() {
                    break;
                }

                let parent_entity = self.id_map.get(parent.as_ref().unwrap());
                if let Some(parent_entity) = parent_entity {
                    let ui = world.get::<UIComponent>(*parent_entity).unwrap();
                    let transform = world.get::<TransformComponent>(*parent_entity).unwrap();

                    parent = ui.parent_id.clone();
                    parent_transform = transform.clone();

                    if let UIType::Container(container) = &ui.ui_type {
                        match container.width {
                            ScreenCoordinate::Pixels(value) => {
                                parent_width = value;
                            }
                            ScreenCoordinate::Percentage(value) => {
                                parent_width = parent_width * value as f32 * 0.01;
                            }
                        };

                        match container.height {
                            ScreenCoordinate::Pixels(value) => {
                                parent_height = value;
                            }
                            ScreenCoordinate::Percentage(value) => {
                                parent_height = parent_height * value as f32 * 0.01;
                            }
                        };
                    }
                } else {
                    error!(
                        "UI ID '{}' does not correspond to any created entity",
                        ui.parent_id.as_ref().unwrap()
                    );
                }
            }

            //info!(transform.translation.y, parent_transform.translation.y);
            match &ui.ui_type {
                UIType::Container(container) => {
                    let mut vector_of_quads_and_z = vec![];

                    let height = match container.height {
                        ScreenCoordinate::Pixels(value) => value,
                        ScreenCoordinate::Percentage(value) => {
                            parent_height * value as f32 * 0.01 - container.border.width * 2.0
                        }
                    };

                    let width = match container.width {
                        ScreenCoordinate::Pixels(value) => value,
                        ScreenCoordinate::Percentage(value) => {
                            parent_width * value as f32 * 0.01 - container.border.width * 2.0
                        }
                    };

                    // Anchor: Top-left corner
                    let bounds = Rect {
                        min: [
                            transform.translation.x + parent_transform.translation.x,
                            transform.translation.y + parent_transform.translation.y,
                        ]
                        .into(),
                        max: [
                            transform.translation.x + width + parent_transform.translation.x,
                            transform.translation.y + height + parent_transform.translation.y,
                        ]
                        .into(),
                    };

                    let mut quad_rect = bounds.clone();

                    quad_rect.min.y -= (bounds.max.y - bounds.min.y) + container.border.width;
                    quad_rect.max.y -= (bounds.max.y - bounds.min.y) + container.border.width;

                    quad_rect.min.x += container.border.width;
                    quad_rect.max.x += container.border.width;

                    // Border is technically a bigger rectangle under our container
                    let border_rect = Rect {
                        min: quad_rect.min
                            - Vector2::new(container.border.width, container.border.width),
                        max: quad_rect.max
                            + Vector2::new(container.border.width, container.border.width),
                    };

                    // Pixel space to normalized device coordinates:
                    // ndc_x = ((pixel_x / screen_width) * 2) - 1
                    // ndc_y = ((pixel_y / screen_height) * 2) + 1 -> We want top left to be 0,0
                    // (ndc rectangle (scaled) + ndc translation) - 1
                    let ndc_rect = Rect {
                        min: Vector2 {
                            x: ((((quad_rect.min.x * transform.scale.x
                                / self.window_container.width)
                                * 2.0)
                                + (((transform.translation.x + parent_transform.translation.x)
                                    / self.window_container.width)
                                    * 2.0))
                                - 1.0),
                            y: ((((quad_rect.min.y * transform.scale.y
                                / self.window_container.height)
                                * 2.0)
                                - (((transform.translation.y + parent_transform.translation.y)
                                    / self.window_container.height)
                                    * 2.0)
                                + 2.0)
                                - 1.0),
                        },
                        max: Vector2 {
                            x: ((((quad_rect.max.x * transform.scale.x
                                / self.window_container.width)
                                * 2.0)
                                + (((transform.translation.x + parent_transform.translation.x)
                                    / self.window_container.width)
                                    * 2.0))
                                - 1.0),
                            y: ((((quad_rect.max.y * transform.scale.y
                                / self.window_container.height)
                                * 2.0)
                                - (((transform.translation.y + parent_transform.translation.y)
                                    / self.window_container.height)
                                    * 2.0)
                                + 2.0)
                                - 1.0),
                        },
                    };

                    let ndc_rect_border = Rect {
                        min: Vector2 {
                            x: ((((border_rect.min.x * transform.scale.x
                                / self.window_container.width)
                                * 2.0)
                                + (((transform.translation.x + parent_transform.translation.x)
                                    / self.window_container.width)
                                    * 2.0))
                                - 1.0),
                            y: ((((border_rect.min.y * transform.scale.y
                                / self.window_container.height)
                                * 2.0)
                                - (((transform.translation.y + parent_transform.translation.y)
                                    / self.window_container.height)
                                    * 2.0)
                                + 2.0)
                                - 1.0),
                        },
                        max: Vector2 {
                            x: ((((border_rect.max.x * transform.scale.x
                                / self.window_container.width)
                                * 2.0)
                                + (((transform.translation.x + parent_transform.translation.x)
                                    / self.window_container.width)
                                    * 2.0))
                                - 1.0),
                            y: ((((border_rect.max.y * transform.scale.y
                                / self.window_container.height)
                                * 2.0)
                                - (((transform.translation.y + parent_transform.translation.y)
                                    / self.window_container.height)
                                    * 2.0)
                                + 2.0)
                                - 1.0),
                        },
                    };

                    let uvs = Rect {
                        min: [0.0, 0.0].into(),
                        max: [1.0, 1.0].into(),
                    };

                    let quad = Primitive::new_quad(
                        ndc_rect,
                        uvs,
                        container.color,
                        transform.translation.z as u16,
                    );
                    let quad_border = Primitive::new_quad(
                        ndc_rect_border,
                        uvs,
                        container.border.color,
                        transform.translation.z as u16 - 1,
                    );

                    let (vert, _) = quad.data();
                    let (vert_border, _) = quad_border.data();

                    vector_of_quads_and_z.append(&mut vec![
                        (vert_border, transform.translation.z as u16),
                        (vert, transform.translation.z as u16 + 1),
                    ]);

                    let data = UIQuadData {
                        changed: true,
                        id: ui.id.clone(),
                        vertices: vector_of_quads_and_z,
                        ui_type: ui.ui_type.clone(),
                    };

                    ui_quad_data_vec.push(data);
                }
                UIType::Text(text) => {
                    parent_transform.translation.x *= 2.0;
                    parent_transform.translation.y *= 2.0;

                    let max_width = if text.max_width < parent_width {
                        parent_transform.translation.x + text.max_width
                    } else {
                        parent_transform.translation.x + parent_width
                    };

                    let mut vertices = vec![];

                    let font = self.fonts.get(&text.font);

                    if let Some(font) = font {
                        // Unwrap safety: in order to generate the atlas the data has already been parsed successfully
                        let font_face = ttf_parser::Face::parse(&font.font_data, 0).unwrap();

                        let units_per_em = font_face.units_per_em() as f32;

                        // Conversion factor from font units to pixels
                        let pixels_per_unit = transform.scale.x / units_per_em;

                        let question_mark_id = font_face
                            .glyph_index('?')
                            .expect("How TF can this font not have '?'");
                        let qmark_plane_bounds =
                            font_face.glyph_bounding_box(question_mark_id).unwrap();

                        let characters = text.text.chars();
                        let characters_count = text.text.len();

                        // X coordinate
                        let mut x = 0f32;

                        // Font size scale
                        let fs_scale =
                            1.0 / (font_face.ascender() as f32 - font_face.descender() as f32);

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
                                y -= fs_scale * font_face.line_gap() as f32 + text.line_spacing;
                                continue;
                            }

                            // Check to see if there are any more character and get the spacing in between to advance the x by
                            if character == ' ' {
                                let mut advance = space_glyph_advance;

                                if index < characters_count - 1 {
                                    let next_char = text.text.as_bytes()[index + 1] as char;
                                    let next_char_id = font_face
                                        .glyph_index(next_char)
                                        .unwrap_or(question_mark_id);
                                    advance = font_face.glyph_hor_advance(next_char_id).unwrap();
                                }

                                x += fs_scale * advance as f32 + text.kerning;
                                continue;
                            }

                            // Add four spaces
                            if character == '\t' {
                                x += 4.0 * (fs_scale * space_glyph_advance as f32 + text.kerning);
                                continue;
                            }

                            // Convert x from font units to pixels using the units_to_pixels factor
                            let x_pixels = x * pixels_per_unit;

                            let max_width = max_width * fs_scale;

                            // Add a check to wrap text if the current x position exceeds container width
                            if x_pixels > max_width {
                                x = 0f32;
                                y -= fs_scale * font_face.line_gap() as f32 + text.line_spacing;
                            }

                            let glyph_id =
                                font_face.glyph_index(character).unwrap_or(question_mark_id);

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

                            // Create a bounding box out of the glyph
                            // Anchor: Top-left corner
                            let mut quad_rect = Rect {
                                min: Vector2::new(
                                    transform.translation.x + parent_transform.translation.x,
                                    transform.translation.y + parent_transform.translation.y,
                                ),
                                max: Vector2::new(
                                    transform.translation.x
                                        + quad_plane_bounds.width() as f32
                                        + parent_transform.translation.x,
                                    transform.translation.y
                                        + quad_plane_bounds.height() as f32
                                        + parent_transform.translation.y,
                                ),
                            };

                            // Position it according to the question mark's plane bounds
                            quad_rect.min.y -= qmark_plane_bounds.height() as f32;
                            quad_rect.max.y -= qmark_plane_bounds.height() as f32;

                            // Scale the bounding box
                            quad_rect.min *= fs_scale;
                            quad_rect.max *= fs_scale;

                            // Position it accordingly
                            quad_rect.min += Vector2::new(x, y);
                            quad_rect.max += Vector2::new(x, y);

                            // Pixel space to normalized device coordinates:
                            // ndc_x = ((pixel_x / screen_width) * 2) - 1
                            // ndc_y = ((pixel_y / screen_height) * 2) + 1 -> We want top left to be 0,0
                            // (ndc rectangle (scaled) + ndc translation) - 1
                            let ndc_rect = Rect {
                                min: Vector2 {
                                    x: ((((quad_rect.min.x * transform.scale.x
                                        / self.window_container.width)
                                        * 2.0)
                                        + (((transform.translation.x
                                            + parent_transform.translation.x)
                                            / self.window_container.width)
                                            * 2.0))
                                        - 1.0),
                                    y: ((((quad_rect.min.y * transform.scale.y
                                        / self.window_container.height)
                                        * 2.0)
                                        - (((transform.translation.y
                                            + parent_transform.translation.y)
                                            / self.window_container.height)
                                            * 2.0)
                                        + 2.0)
                                        - 1.0),
                                },
                                max: Vector2 {
                                    x: ((((quad_rect.max.x * transform.scale.x
                                        / self.window_container.width)
                                        * 2.0)
                                        + (((transform.translation.x
                                            + parent_transform.translation.x)
                                            / self.window_container.width)
                                            * 2.0))
                                        - 1.0),
                                    y: ((((quad_rect.max.y * transform.scale.y
                                        / self.window_container.height)
                                        * 2.0)
                                        - (((transform.translation.y
                                            + parent_transform.translation.y)
                                            / self.window_container.height)
                                            * 2.0)
                                        + 2.0)
                                        - 1.0),
                                },
                            };

                            let quad = Primitive::new_quad(
                                ndc_rect,
                                uvs,
                                text.color,
                                transform.translation.z as u16,
                            );

                            let (vert, _) = quad.data();

                            vertices.push((vert, transform.translation.z as u16));

                            // After each letter add the needed space for the next
                            if index < characters_count - 1 {
                                let advance = font_face.glyph_hor_advance(glyph_id).unwrap();
                                x += fs_scale * advance as f32 + text.kerning;
                            }
                        }

                        let data = UIQuadData {
                            changed: true,
                            id: ui.id.clone(),
                            vertices,
                            ui_type: ui.ui_type.clone(),
                        };

                        ui_quad_data_vec.push(data);
                    }
                }
            }
        }
        (ui_quad_data_vec, indices)
    }
}
