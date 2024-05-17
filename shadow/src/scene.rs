use glam::Vec3;
use wgpu_renderer::scene::Camera;
use winit::keyboard::KeyCode;

pub struct CameraConfig {
    pub sensitivity: f32,
    pub smoothness: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            sensitivity: 1.,
            smoothness: 20.,
        }
    }
}

pub struct ControllableCamera {
    pub camera: Camera,
    target_camera: Camera,
    pub config: CameraConfig,
}

impl ControllableCamera {
    pub fn new(camera: Camera, config: CameraConfig) -> Self {
        Self {
            camera,
            target_camera: camera,
            config,
        }
    }

    #[inline]
    pub fn local_move(&mut self, x: Vec3) {
        self.target_camera.transform.local_move(x);
    }

    pub fn control(&mut self, key: KeyCode) {
        match key {
            KeyCode::KeyW => self.local_move(Vec3::Z * self.config.sensitivity),
            KeyCode::KeyS => self.local_move(Vec3::NEG_Z * self.config.sensitivity),
            KeyCode::KeyA => self.local_move(Vec3::X * self.config.sensitivity),
            KeyCode::KeyD => self.local_move(Vec3::NEG_X * self.config.sensitivity),
            KeyCode::KeyQ => self.local_move(Vec3::NEG_Y * self.config.sensitivity),
            KeyCode::KeyE => self.local_move(Vec3::Y * self.config.sensitivity),
            _ => {}
        }
    }

    pub fn update(&mut self, delta: f32) {
        self.camera.transform.translation = self.camera.transform.translation.lerp(
            self.target_camera.transform.translation,
            self.config.smoothness * delta,
        );
    }
}
