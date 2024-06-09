use cgmath::{Vector2, Vector4};

use super::{buffer::SunBuffer, resources::rect::Rect};

pub trait VertexExt {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

unsafe impl bytemuck::Zeroable for ModelVertex {}
unsafe impl bytemuck::Pod for ModelVertex {}

impl VertexExt for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Quad2DVertex {
    // Position in screen space
    pub position: [f32; 2],
    // Texture coordiante
    pub tex_coords: [f32; 2],
    // Text color
    pub color: [f32; 4],
}

// Interfaces to cast the vertex data to bytes
unsafe impl bytemuck::Zeroable for Quad2DVertex {}
unsafe impl bytemuck::Pod for Quad2DVertex {}

// Describe the layout of the Data in the Text Vertex to the GPU
impl VertexExt for Quad2DVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// Used to set an instance buffer with all text positions so we can draw all text in one draw call
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TextInstance {
    // Position in screen space
    pub position: [f32; 2],
}

// Interfaces to cast the vertex data to bytes
unsafe impl bytemuck::Zeroable for TextInstance {}
unsafe impl bytemuck::Pod for TextInstance {}

#[derive(Debug, Clone)]
pub struct QuadData {
    pub vertices: [Quad2DVertex; 4],
    pub indices: [u32; 6],
}

#[derive(Debug, Clone)]
pub enum Primitive {
    Triangle,
    Quad(QuadData),
    Line,
    Point,
}

impl Primitive {
    /// Takes in a bounding box with the min and max points in NDC space
    /// along with color and the 4 uv coordinates for top right/left bottom right/left
    ///
    /// Returns a Quad variaation of the primitive with Quad Data (vertices and indices)
    pub fn new_quad(bounds: Rect<f32>, uvs: Rect<f32>, color: Vector4<f32>) -> Self {
        let tex_coord_tr: [f32; 2] = [uvs.max.x, uvs.min.y];
        let tex_coord_tl: [f32; 2] = [uvs.min.x, uvs.min.y];
        let tex_coord_bl: [f32; 2] = [uvs.min.x, uvs.max.y];
        let tex_coord_br: [f32; 2] = [uvs.max.x, uvs.max.y];
        let vertices = [
            // Define vertex 0 (top right)
            Quad2DVertex {
                color: color.into(),
                position: (Vector2 {
                    x: bounds.max.x,
                    y: bounds.max.y,
                })
                .into(),
                tex_coords: tex_coord_tr,
            },
            // Define vertex 1 (top left)
            Quad2DVertex {
                color: color.into(),
                position: (Vector2 {
                    x: bounds.min.x,
                    y: bounds.max.y,
                })
                .into(),
                tex_coords: tex_coord_tl,
            },
            // Define vertex 2 (bottom left)
            Quad2DVertex {
                color: color.into(),
                position: (Vector2 {
                    x: bounds.min.x,
                    y: bounds.min.y,
                })
                .into(),
                tex_coords: tex_coord_bl,
            },
            // Define vertex 3 (bottom right)
            Quad2DVertex {
                color: color.into(),
                position: (Vector2 {
                    x: bounds.max.x,
                    y: bounds.min.y,
                })
                .into(),
                tex_coords: tex_coord_br,
            },
        ];

        let indices = [0u32, 1u32, 2u32, 0u32, 2u32, 3u32];

        Primitive::Quad(QuadData { vertices, indices })
    }
}

/// 2D Render interface
pub trait Render2D<'a> {
    fn draw_textured_quad(
        &mut self,
        vb: &'a SunBuffer,
        ib: &'a SunBuffer,
        texture_bind_group: &'a wgpu::BindGroup,
    );
}

// TODO (A40): See how Hazel handle buffers
// Render pass references buffer and texture data from self so we have to indicate
// with lifetime specifiers that self will outlive rpass so that all refs will be valid
impl<'a, 'b> Render2D<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_textured_quad(
        &mut self,
        vb: &'a SunBuffer,
        ib: &'a SunBuffer,
        texture_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, vb.get_buffer().slice(..));
        self.set_index_buffer(ib.get_buffer().slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &texture_bind_group, &vec![]);
        self.draw_indexed(0..6, 0, 0..1);
    }
}
