use std::{
    collections::HashMap,
    io::{BufReader, Cursor},
    ops::Range,
};

use super::{material::SunMaterial, mesh::SunMesh};
use crate::{
    assets::Asset,
    prelude::{
        camera_component::CameraComponent, model_component::ModelComponent,
        transform_component::TransformComponent,
    },
};

use cgmath::SquareMatrix;
use gltf::{json::root::*, Document};
use tracing::error;

#[derive(Debug, Clone)]
pub struct RenderModelDesc {
    pub models: Vec<(ModelComponent, TransformComponent)>,
    pub active_camera: CameraComponent,
}

#[derive(Debug)]
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
        let doc = Document::from_json(root)?;

        let mut meshes: Vec<SunMesh> = Vec::new();
        let mut materials: HashMap<uuid::Uuid, SunMaterial> =
            HashMap::with_capacity(doc.materials().len());

        for scene in doc.scenes() {
            meshes.reserve(scene.nodes().len());
            for node in scene.nodes() {
                let parent_transform = cgmath::Matrix4::<f32>::from_value(1.0);

                let res = SunMesh::from_gltf_node(
                    &mut meshes,
                    node,
                    parent_transform,
                    bind_group_layout,
                    &mut materials,
                    &bin,
                    device,
                    queue,
                );
                match res {
                    Ok(()) => {}
                    Err(err) => error!("{err}"),
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
        mvp_bg: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a SunMesh,
        material: &'a SunMaterial,
        instances: Range<u32>,
        mvp_bg: &'a wgpu::BindGroup,
    );
    fn draw_model(&mut self, model: &'a SunModel, mvp_bg: &'a wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'a SunModel,
        instances: Range<u32>,
        mvp_bg: &'a wgpu::BindGroup,
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
        mvp_bg: &'a wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, mvp_bg);
    }
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a SunMesh,
        material: &'a SunMaterial,
        instances: Range<u32>,
        mvp_bg: &'a wgpu::BindGroup,
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
        self.set_bind_group(0, mvp_bg, &[]);
        self.set_bind_group(1, &material.bind_group, &[]);
        self.set_stencil_reference(1);
        self.draw_indexed(0..mesh.index_count, 0, instances);
    }

    fn draw_model(&mut self, model: &'b SunModel, mvp_bg: &'a wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, mvp_bg);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b SunModel,
        instances: Range<u32>,
        mvp_bg: &'a wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = model.materials.get(&mesh.material).unwrap();
            self.draw_mesh_instanced(mesh, material, instances.clone(), mvp_bg);
        }
    }
}
