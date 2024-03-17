use bevy_ecs::component::Component;
use tracing::error;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub enum PrimitiveType {
    Triangle,
    Quad,
    Line,
    Point,
}

#[derive(Debug, Component, Clone)]
pub struct Primitive {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub primitive_type: PrimitiveType,

    pub uuid: uuid::Uuid,
    pub initialized: bool,

    pub temp_diffuse: Option<String>,
}

impl Primitive {
    pub fn new(
        vertices: Vec<Vertex>,
        indices: Vec<u16>,
        primitive_type: PrimitiveType,
        texture_name: Option<String>,
    ) -> Self {
        let uuid = uuid::Uuid::new_v4();

        let primitive = Self {
            vertices,
            indices,
            primitive_type,
            uuid,
            initialized: false,
            temp_diffuse: texture_name,
        };

        primitive
    }
    pub fn from_args(args: Vec<&str>) -> Option<Self> {
        match args[0] {
            //"pentagon" => Some(Primitive::test_penta()),
            _ => {
                error!("No prefab for the specified shape: {}", args[0]);
                None
            }
        }
    }
}
