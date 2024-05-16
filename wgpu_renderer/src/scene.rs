use glam::{Mat4, Quat, Vec3};
use macros::ShaderData;

use crate::render::ShaderData;

pub struct Camera {
    pub translation: Vec3,
    pub rotation: Quat,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Default)]
pub struct GpuCamera {
    pub view: Mat4,
    pub proj: Mat4,
}

impl ShaderData for GpuCamera {
    fn size() -> usize {
        std::mem::size_of::<Mat4>() * 2
    }

    fn as_raw(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(std::mem::size_of::<Self>());
        buf.extend_from_slice(bytemuck::cast_slice(self.view.as_ref()));
        buf.extend_from_slice(bytemuck::cast_slice(self.proj.as_ref()));
        buf
    }
}

#[derive(ShaderData)]
pub struct DirectionalLight {
    pub position: Vec3,
    pub direction: Vec3,
}
