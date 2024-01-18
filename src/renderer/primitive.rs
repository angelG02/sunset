use bevy_ecs::component::Component;
use tracing::error;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
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
}

impl Primitive {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>, primitive_type: PrimitiveType) -> Self {
        let uuid = uuid::Uuid::new_v4();

        Self {
            vertices,
            indices,
            primitive_type,
            uuid,
            initialized: false,
        }
    }
    pub fn from_args(args: Vec<&str>) -> Option<Self> {
        match args[0] {
            "pentagon" => Some(Primitive::test_penta()),
            _ => {
                error!("No prefab for the specified shape: {}", args[0]);
                None
            }
        }
    }

    pub fn test_penta() -> Self {
        Primitive::new(
            TEST_VERTICES.to_vec(),
            TEST_INDICES.to_vec(),
            PrimitiveType::Triangle,
        )
    }
}

pub const TEST_VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
];

pub const TEST_INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
