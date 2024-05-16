use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::Buffer;

pub trait ShaderData {
    fn min_binding_size() -> Option<NonZeroU64> {
        Some(unsafe { NonZeroU64::new_unchecked(Self::size() as u64) })
    }

    fn size() -> usize;
    fn as_raw(&self) -> Vec<u8>;
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

pub struct GpuMesh {
    pub vertex_count: u32,
    pub vertex_buf: Buffer,
}
