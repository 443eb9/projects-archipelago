use bevy::{
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        component::Component,
        entity::{Entity, EntityHashMap},
        query::{Changed, Or, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes},
            Commands, Query, Res, ResMut, Resource, SystemParamItem,
        },
        world::{FromWorld, World},
    },
    math::{IVec2, UVec2, Vec2, Vec3, Vec4},
    render::{
        mesh::{GpuBufferInfo, GpuMesh, Indices, Mesh, MeshVertexAttribute, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        render_phase::{
            DrawFunctions, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::{
            BindGroup, BindGroupLayout, BindGroupLayoutEntries, BlendState, BufferInitDescriptor,
            BufferUsages, ColorTargetState, ColorWrites, DynamicUniformBuffer, FilterMode,
            FragmentState, GpuArrayBuffer, IndexFormat, MultisampleState, PipelineCache,
            PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
            SpecializedRenderPipelines, TextureFormat, TextureSampleType, VertexBufferLayout,
            VertexFormat, VertexState, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
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
    pub size: UVec2,
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
            &TilemapStorage,
            &GlobalTransform,
        )>,
    >,
) {
    commands.insert_or_spawn_batch(
        tilemaps_query
            .iter()
            .map(
                |(entity, slot_size, animation, texture, storage, transform)| {
                    (
                        entity,
                        ExtractedTilemap {
                            translation: transform.translation(),
                            size: storage.size,
                            slot_size: *slot_size,
                            animation: animation.clone(),
                            texture: texture.cloned(),
                        },
                    )
                },
            )
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
    pub slot_size: Vec2,
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
            // If the y component is NOT -1, then this is a animated tile.
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

// Prepare

#[derive(Resource, Default)]
pub struct TilemapUniformBuffer {
    pub buffer: DynamicUniformBuffer<TilemapUniform>,
}

#[derive(Resource, Default)]
pub struct TilemapAnimationBuffers {
    pub buffers: EntityHashMap<GpuArrayBuffer<u32>>,
}

#[derive(Resource, Default)]
pub struct TilemapBindGroups {
    pub uniforms: Option<BindGroup>,
    pub animations: EntityHashMap<BindGroup>,
}

#[derive(Component)]
pub struct DynamicUniformOffset {
    offset: u32,
}

pub fn prepare_tilemap_bind_groups(
    mut commands: Commands,
    tilemaps_query: Query<(Entity, &ExtractedTilemap)>,
    mut uniform_buffer: ResMut<TilemapUniformBuffer>,
    mut animation_buffers: ResMut<TilemapAnimationBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    uniform_buffer.buffer.clear();
    animation_buffers
        .buffers
        .values_mut()
        .for_each(|b| b.clear());

    for (tilemap_entity, tilemap) in &tilemaps_query {
        let offset = uniform_buffer.buffer.push(&TilemapUniform {
            translation: tilemap.translation,
            slot_size: tilemap.slot_size.0,
        });

        commands
            .entity(tilemap_entity)
            .insert(DynamicUniformOffset { offset });

        let anim_buffer = animation_buffers
            .buffers
            .entry(tilemap_entity)
            .or_insert(GpuArrayBuffer::new(&render_device));

        tilemap.animation.buffer.iter().for_each(|e| {
            anim_buffer.push(*e);
        });
    }

    uniform_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
    animation_buffers
        .buffers
        .values_mut()
        .for_each(|b| b.write_buffer(&render_device, &render_queue));
}

pub const TILEMAP_MESH_ATTR_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("index", 16541000341124, VertexFormat::Uint32x2);
pub const TILEMAP_MESH_ATTR_TINT: MeshVertexAttribute =
    MeshVertexAttribute::new("tint", 454210544510, VertexFormat::Float32x4);
pub const TILEMAP_MESH_ATTR_TEX_IDX: MeshVertexAttribute =
    MeshVertexAttribute::new("texture_index", 541009684125463, VertexFormat::Sint32x2);

#[derive(Clone)]
pub struct MeshTile {
    pub index: UVec2,
    pub texture_index: IVec2,
    pub tint: Vec4,
}

pub struct TilemapMesh {
    pub is_dirty: bool,
    pub is_textured: bool,
    pub mesh: Mesh,
    pub gpu_mesh: Option<GpuMesh>,
    pub tiles: Vec<Option<MeshTile>>,
}

impl TilemapMesh {
    pub fn new(size: UVec2, is_textured: bool) -> Self {
        Self {
            is_dirty: true,
            is_textured,
            mesh: Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            ),
            gpu_mesh: None,
            tiles: vec![None; (size.x * size.y) as usize],
        }
    }
}

#[derive(Resource, Default)]
pub struct TilemapMeshes {
    pub meshes: EntityHashMap<TilemapMesh>,
}

pub fn prepare_tilemap_meshes(
    tilemaps_query: Query<(Entity, &ExtractedTilemap)>,
    mut tilemap_meshes: ResMut<TilemapMeshes>,
    tiles_query: Query<&ExtractedTile>,
    render_device: Res<RenderDevice>,
) {
    for tile in &tiles_query {
        let Ok((tilemap_entity, tilemap)) = tilemaps_query.get(tile.tilemap) else {
            continue;
        };

        let tilemap_mesh = tilemap_meshes
            .meshes
            .entry(tilemap_entity)
            .or_insert_with(|| TilemapMesh::new(tilemap.size, tilemap.texture.is_some()));

        tilemap_mesh.is_dirty = true;
        tilemap_mesh.tiles[(tile.index.y * tilemap.size.x + tile.index.x) as usize] =
            Some(MeshTile {
                index: tile.index,
                texture_index: match &tile.texture {
                    TileTexture::Static(i) => IVec2::new(*i as i32, -1),
                    TileTexture::Animated(a) => IVec2::new(a.start as i32, a.length as i32),
                },
                tint: tile.tint,
            });
    }

    tilemap_meshes.meshes.values_mut().for_each(|mesh| {
        if !mesh.is_dirty {
            return;
        }
        mesh.is_dirty = false;

        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut texture_indices = Vec::new();
        let mut tints = Vec::new();
        let mut vertex_indices = Vec::new();
        let mut v_index = 0;

        for tile in &mesh.tiles {
            let Some(tile) = tile else {
                continue;
            };
            indices.extend_from_slice(&[tile.index, tile.index, tile.index, tile.index]);
            positions.extend_from_slice(&[Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO]);
            tints.extend_from_slice(&[tile.tint, tile.tint, tile.tint, tile.tint]);
            /* 3+--+2
             *  |  |
             * 0+--+1
             */
            vertex_indices.extend_from_slice(&[
                v_index,
                v_index + 1,
                v_index + 2,
                v_index + 2,
                v_index + 3,
                v_index,
            ]);
            v_index += 4;
            if mesh.is_textured {
                texture_indices.extend_from_slice(&[
                    tile.texture_index,
                    tile.texture_index,
                    tile.texture_index,
                    tile.texture_index,
                ]);
            }
        }

        mesh.mesh
            .insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.mesh.insert_attribute(TILEMAP_MESH_ATTR_INDEX, indices);
        mesh.mesh
            .insert_attribute(TILEMAP_MESH_ATTR_TEX_IDX, texture_indices);
        mesh.mesh.insert_attribute(TILEMAP_MESH_ATTR_TINT, tints);
        mesh.mesh.insert_indices(Indices::U32(vertex_indices));

        let num_vertices = mesh.mesh.count_vertices() as u32;
        let num_indices = mesh.mesh.indices().unwrap().len() as u32;

        let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("tilemap_vertex_buffer"),
            contents: &mesh.mesh.get_vertex_buffer_data(),
            usage: BufferUsages::VERTEX,
        });

        let buffer_info =
            mesh.mesh
                .get_index_buffer_bytes()
                .map_or(GpuBufferInfo::NonIndexed, |data| GpuBufferInfo::Indexed {
                    buffer: render_device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("tilemap_indices_buffer"),
                        contents: data,
                        usage: BufferUsages::INDEX,
                    }),
                    count: num_indices,
                    index_format: IndexFormat::Uint32,
                });

        mesh.gpu_mesh = Some(GpuMesh {
            vertex_buffer,
            vertex_count: num_vertices,
            morph_targets: None,
            buffer_info,
            primitive_topology: PrimitiveTopology::TriangleList,
            layout: mesh.mesh.get_mesh_vertex_buffer_layout(),
        });
    });
}

// Draw

type DrawTilemap = (SetItemPipeline, SetTilemapUniformBindGroup<1>);

pub struct SetTilemapUniformBindGroup<const I: usize>;
impl<const I: usize> RenderCommand<Transparent2d> for SetTilemapUniformBindGroup<I> {
    type Param = SRes<TilemapBindGroups>;

    type ViewQuery = ();

    type ItemQuery = Read<DynamicUniformOffset>;

    fn render<'w>(
        _item: &Transparent2d,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        offset: Option<ROQueryItem<'w, Self::ItemQuery>>,
        bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            bind_groups.into_inner().uniforms.as_ref().unwrap(),
            &[offset.unwrap().offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetTilemapAnimationBindGroup<const I: usize>;
impl<const I: usize> RenderCommand<Transparent2d> for SetTilemapAnimationBindGroup<I> {
    type Param = SRes<TilemapBindGroups>;

    type ViewQuery = ();

    type ItemQuery = ();

    fn render<'w>(
        item: &Transparent2d,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_groups.into_inner().animations[&item.entity], &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawTilemapMesh;
impl RenderCommand<Transparent2d> for DrawTilemapMesh {
    type Param = SRes<TilemapMeshes>;

    type ViewQuery = ();

    type ItemQuery = ();

    fn render<'w>(
        item: &Transparent2d,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        tilemap_meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh = tilemap_meshes
            .into_inner()
            .meshes
            .get(&item.entity)
            .unwrap()
            .gpu_mesh
            .as_ref()
            .unwrap();
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        match &mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                count,
                index_format,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..1);
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..mesh.vertex_count, 0..1);
            }
        }
        RenderCommandResult::Success
    }
}
