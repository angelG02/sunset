use crate::renderer::buffer::SunBuffer;

pub struct SunMesh {
    pub name: String,
    pub with_16bit_indices: bool,
    pub id: uuid::Uuid,
    pub vertex_buffer: SunBuffer,
    pub index_buffer: SunBuffer,
    pub index_count: u32,
    pub material: uuid::Uuid,
}
