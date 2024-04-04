use bevy::{
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Changed, Or},
        system::{Commands, Query, Res, ResMut, Resource},
        world::{FromWorld, World},
    },
    math::{UVec2, Vec3, Vec4},
    render::{
        render_phase::{DrawFunctions, RenderPhase},
        render_resource::{
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BlendState,
            ColorTargetState, ColorWrites, FilterMode, FragmentState, MultisampleState,
            PipelineCache, PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, ShaderType, SpecializedMeshPipelines,
            SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
            TextureSampleType, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::Msaa,
        Extract,
    },
    transform::components::GlobalTransform,
    utils::FloatOrd,
};

use super::{
    tilemap::{
        Tile, TileIndex, TileTexture, TilemapAnimation, TilemapSlotSize, TilemapStorage,
        TilemapTexture,
    },
    TILEMAP_SHADER,
};

use bevy::render::render_resource::binding_types as binding;

// Extract

#[derive(Component)]
pub struct ExtractedTilemap {
    pub translation: Vec3,
    pub slot_size: TilemapSlotSize,
    pub animation: TilemapAnimation,
    pub texture: Option<TilemapTexture>,
}

#[derive(Component)]
pub struct ExtractedTile {
    pub tilemap: Entity,
    pub index: UVec2,
    pub texture: TileTexture,
    pub tint: Vec4,
}

pub fn extract_tilemaps(
    mut commands: Commands,
    tilemaps_query: Extract<
        Query<(
            Entity,
            &TilemapSlotSize,
            &TilemapAnimation,
            Option<&TilemapTexture>,
            &GlobalTransform,
        )>,
    >,
) {
    commands.insert_or_spawn_batch(
        tilemaps_query
            .iter()
            .map(|(entity, slot_size, animation, texture, transform)| {
                (
                    entity,
                    ExtractedTilemap {
                        translation: transform.translation(),
                        slot_size: *slot_size,
                        animation: animation.clone(),
                        texture: texture.cloned(),
                    },
                )
            })
            .collect::<Vec<_>>(),
    )
}

pub fn extract_tiles(
    mut commands: Commands,
    tiles_query: Extract<
        Query<(Entity, &Tile, &TileIndex), Or<(Changed<Tile>, Changed<TileIndex>)>>,
    >,
) {
    commands.insert_or_spawn_batch(
        tiles_query
            .iter()
            .map(|(entity, tile, index)| {
                (
                    entity,
                    ExtractedTile {
                        tilemap: tile.tilemap,
                        index: index.0,
                        texture: tile.texture,
                        tint: tile.tint.rgba_linear_to_vec4(),
                    },
                )
            })
            .collect::<Vec<_>>(),
    );
}

// Queue

#[derive(ShaderType)]
pub struct TilemapUniform {
    pub translation: Vec3,
    pub slot_size: UVec2,
}

#[derive(Resource)]
pub struct TilemapPipeline {
    pub layout: BindGroupLayout,
    pub linear_sampler: Sampler,
    pub nearest_sampler: Sampler,
}

impl FromWorld for TilemapPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "tilemap_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    binding::texture_2d(TextureSampleType::Float { filterable: true }),
                    binding::sampler(SamplerBindingType::Filtering),
                    binding::uniform_buffer::<TilemapUniform>(true),
                ),
            ),
        );

        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("tilemap_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("tilemap_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
            layout,
            linear_sampler,
            nearest_sampler,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TilemapPipelineKey {
    pub msaa_sample_count: u32,
    pub has_texture: bool,
}

impl SpecializedRenderPipeline for TilemapPipeline {
    type Key = TilemapPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];
        if key.has_texture {
            shader_defs.push("TEXTURED_TILEMAP".into());
        }

        let vertex_formats = vec![
            // tint
            VertexFormat::Float32x4,
            // index
            VertexFormat::Sint32x2,
            // texture_index
            // If the y component is -1, then this is a animated tile.
            // So we need to consider the x component as start and y as length
            VertexFormat::Sint32x2,
        ];

        RenderPipelineDescriptor {
            label: Some("tilemap_pipeline".into()),
            layout: vec![self.layout.clone()],
            vertex: VertexState {
                shader: TILEMAP_SHADER,
                shader_defs: shader_defs.clone(),
                entry_point: "vertex".into(),
                buffers: vec![VertexBufferLayout::from_vertex_formats(
                    VertexStepMode::Vertex,
                    vertex_formats,
                )],
            },
            fragment: Some(FragmentState {
                shader: TILEMAP_SHADER,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            push_constant_ranges: vec![],
            multisample: MultisampleState {
                count: key.msaa_sample_count,
                ..Default::default()
            },
        }
    }
}

pub fn queue_tilemaps(
    mut views_query: Query<&mut RenderPhase<Transparent2d>>,
    tilemap_query: Query<(Entity, &ExtractedTilemap)>,
    mut sp_pipelines: ResMut<SpecializedRenderPipelines<TilemapPipeline>>,
    tilemap_pipeline: Res<TilemapPipeline>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    draw_functions: ResMut<DrawFunctions<Transparent2d>>,
) {
    for mut transparent_phase in &mut views_query {
        for (tilemap_entity, tilemap) in &tilemap_query {
            let pipeline = sp_pipelines.specialize(
                &pipeline_cache,
                &tilemap_pipeline,
                TilemapPipelineKey {
                    msaa_sample_count: msaa.samples(),
                    has_texture: tilemap.texture.is_some(),
                },
            );

            let draw_function = draw_functions.read().id::<DrawTilemap>();

            transparent_phase.add(Transparent2d {
                sort_key: FloatOrd(tilemap.translation.z),
                entity: tilemap_entity,
                pipeline,
                draw_function,
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}

type DrawTilemap = ();
