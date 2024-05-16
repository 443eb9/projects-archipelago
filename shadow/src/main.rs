use std::borrow::Cow;

use wgpu_renderer::{RendererOutput, WgpuRenderer};
use glam::UVec2;

use wgpu::*;

const TEXTURE_DIM: UVec2 = UVec2::splat(512);

async fn run() {
    let mut renderer = WgpuRenderer::new(
        RendererOutput::Image,
        TEXTURE_DIM,
        ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/scene.wgsl"))),
    )
    .await;

    let render_target = renderer.device().create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: TEXTURE_DIM.x,
            height: TEXTURE_DIM.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[TextureFormat::Rgba8Unorm],
    });
    let target_view = render_target.create_view(&TextureViewDescriptor::default());

    renderer.set_render_target(&render_target, &target_view);
    renderer.load_obj("assets/hung_mesh.obj");

    renderer.draw();
    renderer.save_result("render_output.png").await;
}

fn main() {
    pollster::block_on(run());
}
