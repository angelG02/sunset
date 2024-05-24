use std::{borrow::Cow, collections::HashMap};

use cgmath::SquareMatrix;
use image::ImageFormat;
use tracing::{error, info};

use crate::{
    prelude::{
        primitive::Vertex,
        resources::{material::SunMaterial, model::ModelCreationError, texture::SunTexture},
    },
    renderer::buffer::SunBuffer,
};

#[derive(Debug)]

pub struct SunMesh {
    pub name: String,
    pub id: uuid::Uuid,

    pub with_16bit_indices: bool,

    pub vertex_buffer: SunBuffer,
    pub index_buffer: SunBuffer,
    pub index_count: u32,

    pub material: uuid::Uuid,
}

impl SunMesh {
    pub fn from_gltf_node(
        meshes: &mut Vec<SunMesh>,
        node: gltf::Node<'_>,
        parent_transform: cgmath::Matrix4<f32>,
        bind_group_layout: &wgpu::BindGroupLayout,
        materials: &mut HashMap<uuid::Uuid, SunMaterial>,
        bin: &Cow<'_, [u8]>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<()> {
        if let Some(mesh) = node.mesh() {
            let global_transform: cgmath::Matrix4<f32> =
                parent_transform * Into::<cgmath::Matrix4<f32>>::into(node.transform().matrix());

            let name = mesh.name().unwrap_or("Unnamed_Mesh");
            if global_transform.determinant() < 0.0 {
                info!(
                    "name: {} determinant: {:?}",
                    name,
                    global_transform.determinant()
                );
            }

            let id = uuid::Uuid::new_v4();

            let mut with_16bit_indices = false;

            let material_id = uuid::Uuid::new_v4();

            let num_vertices = mesh
                .primitives()
                .nth(0)
                .unwrap()
                .attributes()
                .nth(0)
                .unwrap()
                .1
                .count();

            // Initialize array for the various components of the mesh
            // with the vertices count to prevent reallocation on resizing
            let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
            let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
            let mut tex_coords: Vec<[f32; 2]> = Vec::with_capacity(num_vertices);
            let mut indices_u32: Vec<u32> = Vec::with_capacity(num_vertices);
            let mut indices_u16: Vec<u16> = Vec::with_capacity(num_vertices);

            // Vertex array creation
            for primitive in mesh.primitives() {
                for (semantic, accessor) in primitive.attributes() {
                    let Some(buffer_view) = accessor.view() else {
                        return Err(anyhow::Error::new(
                            ModelCreationError::UnsupportedSparseAccessor(name.to_owned()),
                        ));
                    };
                    let data =
                        &bin[buffer_view.offset()..buffer_view.offset() + buffer_view.length()];

                    match semantic {
                        gltf::Semantic::Positions => {
                            for chunk in data.chunks(
                                3 * std::mem::size_of::<f32>() + buffer_view.stride().unwrap_or(0),
                            ) {
                                let position = [
                                    f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
                                    f32::from_ne_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
                                    f32::from_ne_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]),
                                ];

                                let position = cgmath::Vector4::<f32>::new(
                                    position[0],
                                    position[1],
                                    position[2],
                                    1.0,
                                );

                                let position = global_transform * position;

                                positions.push([position[0], position[1], position[2]]);
                            }
                        }
                        gltf::Semantic::Normals => {
                            for chunk in data.chunks_exact(3 * std::mem::size_of::<f32>()) {
                                let normal = [
                                    f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
                                    f32::from_ne_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
                                    f32::from_ne_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]),
                                ];
                                normals.push(normal);
                            }
                        }
                        gltf::Semantic::TexCoords(_index) => {
                            for chunk in data.chunks_exact(2 * std::mem::size_of::<f32>()) {
                                let tex_coord = [
                                    f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
                                    f32::from_ne_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]),
                                ];
                                tex_coords.push(tex_coord);
                            }
                        }
                        _ => {}
                    }
                }

                // Index array creation
                let Some(indices_accessor) = primitive.indices() else {
                    return Err(anyhow::Error::new(ModelCreationError::MissingIndexBuffer(
                        name.to_owned(),
                    )));
                };

                let Some(buffer_view) = indices_accessor.view() else {
                    return Err(anyhow::Error::new(ModelCreationError::MissingIndexBuffer(
                        name.to_owned(),
                    )));
                };

                let data = &bin[buffer_view.offset()..buffer_view.offset() + buffer_view.length()];

                match indices_accessor.data_type() {
                    gltf::accessor::DataType::U16 => {
                        for chunk in data.chunks_exact(std::mem::size_of::<u16>()) {
                            let index = u16::from_ne_bytes([chunk[0], chunk[1]]);
                            indices_u16.push(index);
                            with_16bit_indices = true;
                        }
                    }
                    gltf::accessor::DataType::U32 => {
                        for chunk in data.chunks_exact(std::mem::size_of::<u32>()) {
                            let index =
                                u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            indices_u32.push(index);
                        }
                    }
                    _ => {
                        unreachable!()
                    }
                }

                // Material creation
                use gltf::image::*;

                // If texture images and buffers are present, create gpu views and samplers from them
                if let Some(base_color_texture_info) = primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_texture()
                {
                    let material_name = primitive.material().name().unwrap_or("Unnamed_Material");

                    let src = base_color_texture_info.texture().source().source();

                    match src {
                        Source::View { view, mime_type } => {
                            let data = &bin[view.offset()..view.offset() + view.length()];
                            let format: ImageFormat = match mime_type {
                                "image/png" => ImageFormat::Png,
                                "image/jpeg" => ImageFormat::Jpeg,
                                _ => {
                                    unreachable!()
                                }
                            };

                            let diffuse_texture = SunTexture::from_bytes(
                                "Diffuse Texture",
                                device,
                                queue,
                                data,
                                format,
                            )?;

                            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: None,
                                layout: bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(
                                            &diffuse_texture.view,
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(
                                            &diffuse_texture.sampler,
                                        ),
                                    },
                                ],
                            });

                            materials.insert(
                                material_id,
                                SunMaterial {
                                    name: material_name.to_owned(),
                                    id: material_id,
                                    diffuse_texture,
                                    bind_group,
                                },
                            );
                        }
                        Source::Uri {
                            uri: _,
                            mime_type: _,
                        } => {
                            unreachable!()
                        }
                    }
                } else {
                    // Otherwise, create 1x1 views and samplers from the base color factor
                    let base_color_factor = primitive
                        .material()
                        .pbr_metallic_roughness()
                        .base_color_factor();

                    let diffuse_texture = SunTexture::from_color(
                        "Diffuse Texture",
                        device,
                        queue,
                        &base_color_factor,
                    );

                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                            },
                        ],
                    });

                    materials.insert(
                        material_id,
                        SunMaterial {
                            name: "base_color".to_owned(),
                            id: material_id,
                            diffuse_texture,
                            bind_group,
                        },
                    );
                };
            }

            let vertices = (0..positions.len())
                .map(|i| Vertex {
                    position: positions[i],
                    tex_coords: *tex_coords.get(i).unwrap_or(&[0.0, 0.0]),
                    normal: *normals.get(i).unwrap_or(&[0.0, 0.0, 0.0]),
                })
                .collect::<Vec<_>>();

            let vb = SunBuffer::new_with_data(
                "Vertex buffer",
                wgpu::BufferUsages::VERTEX,
                bytemuck::cast_slice(&vertices),
                device,
            );

            let ib = SunBuffer::new_with_data(
                "Index buffer",
                wgpu::BufferUsages::INDEX,
                if with_16bit_indices {
                    bytemuck::cast_slice(&indices_u16)
                } else {
                    bytemuck::cast_slice(&indices_u32)
                },
                device,
            );

            let mesh = SunMesh {
                id,
                name: name.to_owned(),
                with_16bit_indices,
                vertex_buffer: vb,
                index_buffer: ib,
                index_count: if with_16bit_indices {
                    indices_u16.len() as u32
                } else {
                    indices_u32.len() as u32
                },
                material: material_id,
            };
            meshes.push(mesh);
        }
        for child in node.children() {
            // Order matters! The transform sapce is determined by the left-most transform
            // The order then is from right to left

            // Parent * Child
            let parent_trans: cgmath::Matrix4<f32> =
                parent_transform * Into::<cgmath::Matrix4<f32>>::into(node.transform().matrix());

            let res = Self::from_gltf_node(
                meshes,
                child,
                parent_trans,
                bind_group_layout,
                materials,
                bin,
                device,
                queue,
            );

            match res {
                Ok(()) => {}
                Err(err) => error!("{err}"),
            }
        }
        Ok(())
    }
}
