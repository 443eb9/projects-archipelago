use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Transform {
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.rotation.mul_vec3(p) + self.translation
    }

    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.translation)
    }

    pub fn local_move(&mut self, x: Vec3) {
        self.translation += self.rotation.mul_vec3(x);
    }

    pub fn rotate(&mut self, axis: Vec3, angle: f32) {
        self.rotation = self.rotation.mul_quat(Quat::from_axis_angle(axis, angle));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub transform: Transform,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    pub translation: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
}
