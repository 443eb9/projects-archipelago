use std::{
    borrow::Cow,
    f32::consts::FRAC_PI_4,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use glam::{UVec2, Vec3};
use wgpu::{ShaderSource, TextureFormat};
use wgpu_renderer::{
    scene::{Camera, DirectionalLight, Transform},
    RendererConfig, WgpuSurfaceRenderer,
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowAttributes, WindowId},
};

use crate::scene::{CameraConfig, ControllableCamera};

pub struct Application<'w> {
    pub renderer: WgpuSurfaceRenderer<'w>,
    window: Arc<Window>,
    fps: f32,

    main_camera: Arc<Mutex<ControllableCamera>>,
}

impl<'w> Application<'w> {
    pub async fn new(event_loop: &EventLoop<()>, dim: UVec2, fps: f32) -> Self {
        #[allow(deprecated)]
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(Size::Physical(PhysicalSize::new(dim.x, dim.y))),
                )
                .unwrap(),
        );

        let mut renderer = WgpuSurfaceRenderer::new(
            window.clone(),
            dim,
            ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/scene.wgsl"))),
            Some(RendererConfig {
                primary_target_format: TextureFormat::Bgra8UnormSrgb,
                ..Default::default()
            }),
        )
        .await;

        let main_camera = ControllableCamera::new(
            Camera {
                transform: Transform::default(),
                aspect_ratio: dim.x as f32 / dim.y as f32,
                fov: FRAC_PI_4,
                near: 0.1,
                far: 1000.,
            },
            CameraConfig::default(),
        );

        renderer.renderer_mut().set_camera(&main_camera.camera);

        renderer.renderer_mut().dir_lights.push(DirectionalLight {
            translation: Vec3::new(10., 20., 0.),
            direction: Vec3::new(-1., -1.2, 1.).normalize(),
            color: Vec3::ONE,
        });
        renderer.renderer_mut().load_obj("assets/icosphere.obj");
        renderer.renderer_mut().write_scene();

        Self {
            renderer,
            window,
            fps,

            main_camera: Arc::new(Mutex::new(main_camera)),
        }
    }

    pub fn run(&self) {
        let window = self.window.clone();
        let main_camera = self.main_camera.clone();
        let mut delta = 0.;

        thread::spawn(move || loop {
            let start = std::time::Instant::now();

            window.request_redraw();
            main_camera.lock().unwrap().update(delta);

            delta = start.elapsed().as_secs_f32();
        });
    }
}

impl<'w> ApplicationHandler for Application<'w> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let Ok(main_camera) = self.main_camera.lock() else {
                    return;
                };
                self.renderer.renderer_mut().set_camera(&main_camera.camera);
                self.renderer.renderer_mut().write_scene();
                self.renderer.draw();
            }
            WindowEvent::CloseRequested => std::process::exit(0),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                let Ok(mut main_camera) = self.main_camera.lock() else {
                    return;
                };
                match event.physical_key {
                    PhysicalKey::Code(key) => main_camera.control(key),
                    PhysicalKey::Unidentified(_) => {}
                }
            },
            _ => {}
        }
    }
}
