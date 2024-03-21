use uuid::Uuid;
use wgpu::{BindGroupLayout, Device, Queue};

use super::texture::SunTexture;

pub struct SunMaterial {
    pub name: String,
    pub id: uuid::Uuid,
    pub diffuse_texture: SunTexture,
    pub bind_group: wgpu::BindGroup,
}

impl SunMaterial {
    pub fn from_bytes(
        data: &[u8],
        name: &str,
        id: Option<uuid::Uuid>,
        device: &Device,
        queue: &Queue,
        bind_group_layout: &BindGroupLayout,
    ) -> anyhow::Result<SunMaterial> {
        let diffuse_texture = SunTexture::from_bytes(device, queue, data, "Diffuse Texture")?;

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

        Ok(Self {
            name: name.to_owned(),
            id: id.unwrap_or(Uuid::new_v4()),
            diffuse_texture,
            bind_group,
        })
    }
}
