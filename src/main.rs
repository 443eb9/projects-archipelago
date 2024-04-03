use bevy::{
    app::{App, Startup},
    core_pipeline::core_2d::Camera2dBundle,
    ecs::system::Commands,
    DefaultPlugins,
};
use ch1_custom_material::Chapter1Plugin;

mod ch1_custom_material;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Chapter1Plugin))
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
