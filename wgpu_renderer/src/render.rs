use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::Buffer;

pub trait ShaderData: Sized {
    fn as_raw(&self) -> Vec<u8>;

    fn min_binding_size() -> Option<NonZeroU64> {
        Some(unsafe { NonZeroU64::new_unchecked(std::mem::size_of::<Self>() as u64) })
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

pub struct GpuMesh {
    pub vertex_count: u32,
    pub vertex_buf: Buffer,
}

#[derive(Default, Debug)]
pub struct GpuCamera {
    pub view: Mat4,
    pub proj: Mat4,
}

impl ShaderData for GpuCamera {
    fn as_raw(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(std::mem::size_of::<Self>());
        buf.extend_from_slice(bytemuck::cast_slice(self.view.as_ref()));
        buf.extend_from_slice(bytemuck::cast_slice(self.proj.as_ref()));
        buf
    }
}

#[derive(Default, Debug)]
pub struct GpuDirectionalLight {
    pub translation: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
}

impl ShaderData for GpuDirectionalLight {
    fn as_raw(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(std::mem::size_of::<Self>());
        buf.extend_from_slice(bytemuck::cast_slice(self.translation.as_ref()));
        buf.extend_from_slice(bytemuck::cast_slice(self.direction.as_ref()));
        buf.extend_from_slice(bytemuck::cast_slice(self.color.as_ref()));
        buf.extend_from_slice(&[0; 12]);
        buf
    }
}
