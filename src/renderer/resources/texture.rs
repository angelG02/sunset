use anyhow::*;
use image::{GenericImageView, ImageFormat, Rgba, RgbaImage};

#[derive(Debug)]

pub struct SunTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,

    // TODO (A40): Sepatate Image from Texture
    pub data: Vec<u8>,
    pub format: wgpu::TextureFormat,
    pub size: wgpu::Extent3d,

    pub name: String,
    pub uuid: uuid::Uuid,
}

impl SunTexture {
    pub fn from_bytes(
        label: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        format: ImageFormat,
    ) -> Result<Self> {
        let img = image::load_from_memory_with_format(bytes, format)?;
        Self::from_image(label, device, queue, &img)
    }

    pub fn from_image(
        label: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        // NOTE (A40): This might be an incorrect assumption for some imaeg formats (like jpeg)
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            uuid: uuid::Uuid::new_v4(),
            name: label.to_owned(),
            size,
            format,
            data: rgba.to_vec(),
        })
    }

    pub fn from_color(
        label: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color: &[f32; 4],
    ) -> Self {
        let r = (color[0] * 255.0).round() as u8;
        let g = (color[1] * 255.0).round() as u8;
        let b = (color[2] * 255.0).round() as u8;
        let a = (color[3] * 255.0).round() as u8;
        let rgba = RgbaImage::from_pixel(1, 1, Rgba([r, g, b, a]));

        // NOTE (A40): This might be an incorrect assumption for some imaeg formats (like jpeg)
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 1),
                rows_per_image: Some(1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            uuid: uuid::Uuid::new_v4(),
            name: label.to_owned(),
            size,
            format,
            data: rgba.to_vec(),
        }
    }

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32FloatStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);
        let format = wgpu::TextureFormat::Depth32FloatStencil8;

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("depth_texture_view"),
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            name: label.to_string(),
            uuid: uuid::Uuid::new_v4(),
            size,
            format,
            data: Vec::new(),
        }
    }
}
