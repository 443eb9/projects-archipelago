use bevy::{
    app::{App, Startup, Update},
    core_pipeline::core_2d::Camera2dBundle,
    ecs::{
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, ResMut},
    },
    input::{keyboard::KeyCode, ButtonInput},
    prelude::PluginGroup,
    render::view::screenshot::ScreenshotManager,
    window::{Window, WindowPlugin, WindowResolution},
    DefaultPlugins,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use render::NoisePlugin;

mod render;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // resizable: false,
                    // resolution: WindowResolution::new(205., 205.),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            NoisePlugin,
            WorldInspectorPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, screenshot)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn screenshot(
    window: Query<Entity, With<Window>>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::F12) {
        screenshot_manager
            .save_screenshot_to_disk(window.single(), "generated/noise.png")
            .unwrap();
    }
}
