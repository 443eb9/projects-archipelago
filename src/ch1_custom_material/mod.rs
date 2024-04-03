use bevy::{
    app::{App, Plugin},
    asset::{Asset, Handle},
    reflect::TypePath,
    render::{
        color::Color,
        render_resource::{AsBindGroup, Buffer},
        texture::Image,
    },
};

pub struct Chapter1Plugin;

impl Plugin for Chapter1Plugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Asset, AsBindGroup, TypePath)]
struct MyMaterial {
    #[uniform(0)]
    pub color: Color,

    #[texture(
        1,
        dimension = "2d",
        sample_type = "float",
        multisampled = true,
        filterable = true
    )]
    #[sampler(2, sampler_type = "filtering")]
    pub texture: Option<Handle<Image>>,

    #[storage_texture(3, dimension = "2d")]
    pub storage_texture: Handle<Image>,

    #[storage(4, read_only, buffer)]
    pub buffer: Buffer,
}
