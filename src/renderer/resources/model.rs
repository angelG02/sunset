use std::{
    collections::HashMap,
    io::{BufReader, Cursor},
    ops::Range,
};

use super::{material::SunMaterial, mesh::SunMesh};
use crate::{
    assets::Asset,
    prelude::{buffer::SunBuffer, primitive::Vertex, resources::texture::SunTexture},
};

use gltf::{json::root::*, Document};

pub struct SunModel {
    pub id: uuid::Uuid,
    pub meshes: Vec<SunMesh>,
    pub materials: HashMap<uuid::Uuid, SunMaterial>,
}

impl SunModel {
    pub fn from_glb(
        asset: &Asset,
        bind_group_layout: &wgpu::BindGroupLayout,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) -> anyhow::Result<Self> {
        let cursor = Cursor::new(&asset.data);
        let reader = BufReader::new(cursor);

        let glb_model = gltf::Glb::from_reader(reader)?;
        let Some(bin) = glb_model.bin else {
            return Err(anyhow::Error::new(ModelCreationError::BinSectionNotFound(
                asset.name.clone(),
            )));
        };

        let root = Root::from_slice(&glb_model.json)?;
        let doc = Document::from_json_without_validation(root);

        let mut meshes: Vec<SunMesh> = Vec::new();
        let mut materials: HashMap<uuid::Uuid, SunMaterial> =
            HashMap::with_capacity(doc.materials().len());

        for scene in doc.scenes() {
            meshes.reserve(scene.nodes().len());
            for node in scene.nodes() {
                if let Some(mesh) = node.mesh() {
                    let id = uuid::Uuid::new_v4();
                    let name = mesh.name().unwrap_or("Unnamed_Mesh");
                    let material_id = uuid::Uuid::new_v4();
                    let mut with_16bit_indices = false;

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
                            let data = &bin
                                [buffer_view.offset()..buffer_view.offset() + buffer_view.length()];

                            match semantic {
                                gltf::Semantic::Positions => {
                                    for chunk in data.chunks_exact(3 * std::mem::size_of::<f32>()) {
                                        let position = [
                                            f32::from_ne_bytes([
                                                chunk[0], chunk[1], chunk[2], chunk[3],
                                            ]),
                                            f32::from_ne_bytes([
                                                chunk[4], chunk[5], chunk[6], chunk[7],
                                            ]),
                                            f32::from_ne_bytes([
                                                chunk[8], chunk[9], chunk[10], chunk[11],
                                            ]),
                                        ];
                                        positions.push(position);
                                    }
                                }
                                gltf::Semantic::Normals => {
                                    for chunk in data.chunks_exact(3 * std::mem::size_of::<f32>()) {
                                        let normal = [
                                            f32::from_ne_bytes([
                                                chunk[0], chunk[1], chunk[2], chunk[3],
                                            ]),
                                            f32::from_ne_bytes([
                                                chunk[4], chunk[5], chunk[6], chunk[7],
                                            ]),
                                            f32::from_ne_bytes([
                                                chunk[8], chunk[9], chunk[10], chunk[11],
                                            ]),
                                        ];
                                        normals.push(normal);
                                    }
                                }
                                gltf::Semantic::TexCoords(_index) => {
                                    for chunk in data.chunks_exact(2 * std::mem::size_of::<f32>()) {
                                        let tex_coord = [
                                            f32::from_ne_bytes([
                                                chunk[0], chunk[1], chunk[2], chunk[3],
                                            ]),
                                            f32::from_ne_bytes([
                                                chunk[4], chunk[5], chunk[6], chunk[7],
                                            ]),
                                        ];
                                        tex_coords.push(tex_coord);
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Index array creation
                        let Some(indices_accessor) = primitive.indices() else {
                            return Err(anyhow::Error::new(
                                ModelCreationError::MissingIndexBuffer(name.to_owned()),
                            ));
                        };

                        let Some(buffer_view) = indices_accessor.view() else {
                            return Err(anyhow::Error::new(
                                ModelCreationError::MissingIndexBuffer(name.to_owned()),
                            ));
                        };

                        let data =
                            &bin[buffer_view.offset()..buffer_view.offset() + buffer_view.length()];

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
                                    let index = u32::from_ne_bytes([
                                        chunk[0], chunk[1], chunk[2], chunk[3],
                                    ]);
                                    indices_u32.push(index);
                                }
                            }
                            _ => {
                                unreachable!()
                            }
                        }

                        // Material creation
                        use gltf::image::*;

                        let Some(base_color_texture_info) = primitive
                            .material()
                            .pbr_metallic_roughness()
                            .base_color_texture()
                        else {
                            return Err(anyhow::Error::new(
                                ModelCreationError::BaseColorTextureNotFound(name.to_owned()),
                            ));
                        };

                        let material_name =
                            primitive.material().name().unwrap_or("Unnamed_Material");

                        let src = base_color_texture_info.texture().source().source();

                        match src {
                            Source::View { view, mime_type: _ } => {
                                let data = &bin[view.offset()..view.offset() + view.length()];

                                let diffuse_texture =
                                    SunTexture::from_bytes(device, queue, data, "Diffuse Texture")?;

                                let bind_group =
                                    device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    }

                    let vertices = (0..positions.len())
                        .map(|i| Vertex {
                            position: positions[i],
                            tex_coords: tex_coords[i],
                            normal: normals[i],
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
                        with_16bit_indices,
                        name: name.to_owned(),
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
            }
        }

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            meshes,
            materials,
        })
    }
}

#[derive(Debug)]
pub enum ModelCreationError {
    UnsupportedSparseAccessor(String),
    BinSectionNotFound(String),
    MissingIndexBuffer(String),
    BaseColorTextureNotFound(String),
    IDK(String),
}

impl std::fmt::Display for ModelCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ModelCreationError::UnsupportedSparseAccessor(name) => write!(f, "Sparse Accessors are unsupported at this time! Please provide a model with a valid buffer view! Model name: {}", name),
            ModelCreationError::BinSectionNotFound(name) => write!(f, "Binary section not found! Glb model must contain a bin section with all texture and vertex data! Model name: {}", name),
            ModelCreationError::MissingIndexBuffer(name) => write!(f, "Index buffer not found! Glb model must contain an index buffer! Model name: {}", name),
            ModelCreationError::BaseColorTextureNotFound(name) => write!(f, "Base color texture not found! Glb model must contain a base color texture! Model name: {}", name),
            ModelCreationError::IDK(name) => write!(f, "LMAO IDK BRO Model name: {}", name)
        }
    }
}

impl std::error::Error for ModelCreationError {
    fn description(&self) -> &str {
        match self {
            ModelCreationError::UnsupportedSparseAccessor(_name) => "Sparse Accessors are unsupported at this time! Please provide a model with a valid buffer view!",
            ModelCreationError::BinSectionNotFound(_name) => "Binary section not found!Glb model must contain a bin section with all texture and vertex data!",
            ModelCreationError::MissingIndexBuffer(_name) => "Index buffer not found!Glb model must contain an index buffer!",
            ModelCreationError::BaseColorTextureNotFound(_) => "Base color texture not found! Glb model must contain a base color texture!",
            ModelCreationError::IDK(_name) => "LMAO IDK BRO",
        }
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a SunMesh,
        material: &'a SunMaterial,
        camera_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a SunMesh,
        material: &'a SunMaterial,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model(&mut self, model: &'a SunModel, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'a SunModel,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b SunMesh,
        material: &'a SunMaterial,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group);
    }
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a SunMesh,
        material: &'a SunMaterial,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.get_buffer().slice(..));
        self.set_index_buffer(
            mesh.index_buffer.get_buffer().slice(..),
            if mesh.with_16bit_indices {
                wgpu::IndexFormat::Uint16
            } else {
                wgpu::IndexFormat::Uint32
            },
        );
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, &material.bind_group, &[]);
        self.draw_indexed(0..mesh.index_count, 0, instances);
    }

    fn draw_model(&mut self, model: &'b SunModel, camera_bind_group: &'b wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, camera_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b SunModel,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = model.materials.get(&mesh.material).unwrap();
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group);
        }
    }
}
