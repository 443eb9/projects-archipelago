use bevy::{
    app::{App, Plugin, Startup},
    asset::{load_internal_asset, AssetServer, Handle},
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        schedule::IntoSystemConfigs,
        system::{Commands, Res},
    },
    math::{UVec2, Vec2},
    render::{
        color::Color,
        render_phase::AddRenderCommand,
        render_resource::{Shader, SpecializedRenderPipelines},
        ExtractSchedule, Render, RenderApp, RenderSet,
    },
};

use crate::ch3_tilemap::render::{
    DrawTilemap, TilemapAnimationBuffers, TilemapMeshes, TilemapUniformBuffer,
};

use self::{
    render::{queue_tilemaps, TilemapPipeline},
    tilemap::{
        Tile, TileTexture, TilemapAnimation, TilemapSlotSize, TilemapStorage, TilemapTexture,
    },
};

mod render;
mod tilemap;

pub const TILEMAP_SHADER: Handle<Shader> = Handle::weak_from_u128(897641320865040840533184);

pub struct Chapter3Plugin;

impl Plugin for Chapter3Plugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TILEMAP_SHADER,
            "tilemap_shader.wgsl",
            Shader::from_wgsl
        );

        app.add_systems(Startup, tilemap_setup);

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .add_systems(
                ExtractSchedule,
                (render::extract_tilemaps, render::extract_tiles),
            )
            .add_systems(Render, render::queue_tilemaps.in_set(RenderSet::Queue))
            .add_systems(
                Render,
                (
                    render::prepare_tilemap_bind_groups,
                    render::prepare_tilemap_meshes,
                )
                    .in_set(RenderSet::Prepare),
            )
            .init_resource::<TilemapMeshes>()
            .init_resource::<TilemapUniformBuffer>()
            .init_resource::<TilemapAnimationBuffers>()
            .add_render_command::<Transparent2d, DrawTilemap>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<TilemapPipeline>()
            .init_resource::<SpecializedRenderPipelines<TilemapPipeline>>();
    }
}

fn tilemap_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let tilemap = commands.spawn_empty().id();
    let mut anim = TilemapAnimation::default();
    let mut storage = TilemapStorage::new(UVec2::splat(16));

    let sample_anim = anim.add_animation(vec![0, 1, 2, 3], 10);

    for x in 0..8 {
        for y in 0..8 {
            storage.set(
                &mut commands,
                UVec2 { x, y },
                Tile {
                    tilemap,
                    texture: TileTexture::Animated(sample_anim),
                    tint: Color::LIME_GREEN,
                },
            );
        }
    }

    for x in 8..16 {
        for y in 8..16 {
            storage.set(
                &mut commands,
                UVec2 { x, y },
                Tile {
                    tilemap,
                    texture: TileTexture::Static(2),
                    tint: Color::RED,
                },
            );
        }
    }

    commands.entity(tilemap).insert((
        TilemapSlotSize(Vec2::splat(16.)),
        TilemapTexture {
            image: asset_server.load("tiles.png"),
            image_size: UVec2::splat(32),
            tile_size: UVec2::splat(16),
        },
        storage,
        anim,
    ));
}
