use std::marker::PhantomData;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindingResource, Buffer, BufferUsages, Device, Queue,
};

use crate::render::ShaderData;

#[derive(Default)]
pub struct StorageBuffer<T: ShaderData> {
    raw: Vec<u8>,
    buffer: Option<Buffer>,
    changed: bool,
    marker: PhantomData<T>,
}

impl<T: ShaderData> StorageBuffer<T> {
    #[inline]
    pub fn set(&mut self, data: &[T]) {
        self.raw = data.iter().flat_map(|e| e.as_raw()).collect();
        self.changed = true;
    }

    #[inline]
    pub fn push(&mut self, data: &T) {
        self.raw.extend(data.as_raw());
        self.changed = true;
    }

    pub fn write(&mut self, device: &Device, queue: &Queue) {
        let cap = self.buffer.as_ref().map(wgpu::Buffer::size).unwrap_or(0);
        let size = self.raw.len() as u64;

        if self.changed || cap < size {
            self.buffer = Some(device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: &self.raw,
                usage: BufferUsages::STORAGE,
            }));
            self.changed = false;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(&buffer, 0, &self.raw);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.raw.clear();
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        self.buffer.as_ref().map(|b| b.as_entire_binding())
    }
}
