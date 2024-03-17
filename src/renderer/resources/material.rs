use super::texture::SunTexture;

pub struct SunMaterial {
    pub name: String,
    pub id: uuid::Uuid,
    pub diffuse_texture: SunTexture,
    pub bind_group: wgpu::BindGroup,
}
