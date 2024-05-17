use std::{borrow::Cow, f32::consts::FRAC_PI_4};

use app::Application;
use glam::{UVec2, Vec3};

use wgpu_renderer::{
    scene::{Camera, DirectionalLight, Transform},
    WgpuImageRenderer,
};

use wgpu::*;
use winit::event_loop::EventLoop;

mod app;
mod scene;

const TEXTURE_DIM: UVec2 = UVec2::splat(512);
const WINDOW_DIM: UVec2 = UVec2::new(1920, 1080);

async fn render_to_image(dim: UVec2) {
    let mut renderer = WgpuImageRenderer::new(
        dim,
        ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/scene.wgsl"))),
        None,
    )
    .await;

    renderer.renderer_mut().set_camera(&Camera {
        transform: Transform::default(),
        aspect_ratio: dim.x as f32 / dim.y as f32,
        fov: FRAC_PI_4,
        near: 0.1,
        far: 1000.,
    });
    renderer.renderer_mut().dir_lights.push(DirectionalLight {
        translation: Vec3::new(10., 20., 0.),
        direction: Vec3::new(-1., -1.2, 1.).normalize(),
        color: Vec3::ONE,
    });
    renderer.renderer_mut().write_scene();
    renderer.renderer_mut().load_obj("assets/hung_mesh.obj");

    renderer.draw().await;
    renderer.save_result("render_output.png").await;
}

async fn realtime_render(dim: UVec2) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = Application::new(&event_loop, dim, 144.).await;
    app.run();
    event_loop.run_app(&mut app).unwrap();
}

fn main() {
    pollster::block_on(realtime_render(WINDOW_DIM));
}
