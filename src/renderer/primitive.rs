use bevy_ecs::component::Component;
use tracing::error;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
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
}

impl Primitive {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>, primitive_type: PrimitiveType) -> Self {
        let uuid = uuid::Uuid::new_v4();

        let primitive = Self {
            vertices,
            indices,
            primitive_type,
            uuid,
            initialized: false,
        };

        primitive
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
        let primitive = Primitive::new(
            TEST_VERTICES.to_vec(),
            TEST_INDICES.to_vec(),
            PrimitiveType::Triangle,
        );

        primitive
    }
}

pub const TEST_VERTICES: &[Vertex] = &[
    // Changed
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732914],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
    }, // E
];

pub const TEST_INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
