use wgpu::{util::DeviceExt, Device};

use super::primitive::Primitive;

#[derive(Debug, Clone)]
pub struct BufferDesc {
    pub data: Vec<Primitive>,
}

#[derive(Debug)]
pub struct SunBuffer {
    pub label: String,
    pub usage: wgpu::BufferUsages,

    inner: wgpu::Buffer,
    dirty: bool,
}

impl SunBuffer {
    pub fn new_with_data(
        label: &str,
        usage: wgpu::BufferUsages,
        contents: &[u8],
        device: &Device,
    ) -> Self {
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage,
        });

        Self {
            label: label.to_owned(),
            usage,
            inner: buf,
            dirty: false,
        }
    }

    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    pub fn regenerate(self, new_contents: &[u8], device: &Device) -> Self {
        SunBuffer::new_with_data(&self.label, self.usage, new_contents, device)
    }
}
