use bevy::{
    app::{App, Startup},
    asset::Assets,
    core_pipeline::core_2d::Camera2dBundle,
    ecs::system::{Commands, ResMut},
    math::primitives::Rectangle,
    render::{color::Color, mesh::Mesh},
    sprite::{ColorMaterial, ColorMesh2dBundle, Mesh2dHandle},
    DefaultPlugins,
};

mod ch1_custom_material;
mod ch2_post_processing;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // ch1_custom_material::Chapter1Plugin,
            ch2_post_processing::Chapter2Plugin,
        ))
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2dBundle::default(),
        ch2_post_processing::MonochromerSettings { speed: 2. },
    ));

    commands.spawn(ColorMesh2dBundle {
        mesh: Mesh2dHandle(meshes.add(Rectangle::new(200., 150.))),
        material: materials.add(ColorMaterial {
            color: Color::BLUE,
            ..Default::default()
        }),
        ..Default::default()
    });
}
