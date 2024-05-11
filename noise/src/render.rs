use bevy::{
    app::{App, Plugin},
    asset::{load_internal_asset, Handle},
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::{
        query::QueryItem,
        reflect::ReflectResource,
        schedule::IntoSystemConfigs,
        system::{lifetimeless::Read, Query, Res, ResMut, Resource},
        world::{FromWorld, World},
    },
    math::Vec2,
    reflect::Reflect,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, DynamicUniformBuffer, FragmentState, MultisampleState,
            PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Shader, ShaderStages, ShaderType, SpecializedRenderPipeline,
            SpecializedRenderPipelines, StorageBuffer, TextureFormat,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, ViewTarget},
        Render, RenderApp, RenderSet,
    },
};

use bevy::render::render_resource::binding_types as binding;

const NOISE_SHADER: Handle<Shader> = Handle::weak_from_u128(543168451847516852874615024875120);
const NOISE_TYPES_SHADER: Handle<Shader> = Handle::weak_from_u128(894651320489516320);
const HASH_SHADER: Handle<Shader> = Handle::weak_from_u128(798749816004806461564689531);

const VALUE_NOISE_SHADER: Handle<Shader> = Handle::weak_from_u128(7845120894513845124510);
const PERLIN_NOISE_SHADER: Handle<Shader> = Handle::weak_from_u128(487512048512048756120);
const SIMPLEX_NOISE_SHADER: Handle<Shader> = Handle::weak_from_u128(98746510574501685064331);

pub struct NoisePlugin;

impl Plugin for NoisePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, NOISE_SHADER, "noise.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            NOISE_TYPES_SHADER,
            "noise_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, HASH_SHADER, "hash.wgsl", Shader::from_wgsl);

        load_internal_asset!(
            app,
            VALUE_NOISE_SHADER,
            "noise_funcs/value.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PERLIN_NOISE_SHADER,
            "noise_funcs/perlin.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SIMPLEX_NOISE_SHADER,
            "noise_funcs/simplex.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(ExtractResourcePlugin::<NoiseSettings>::default())
            .init_resource::<NoiseSettings>()
            .register_type::<NoiseSettings>();

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<NoiseUniformBuffer>()
            .init_resource::<DomainWarpBuffer>()
            .add_systems(Render, prepare.in_set(RenderSet::Prepare))
            .add_render_graph_node::<ViewNodeRunner<NoiseNode>>(Core2d, NoiseNodeLabel)
            .add_render_graph_edges(Core2d, (Node2d::MainPass, NoiseNodeLabel, Node2d::Bloom));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<NoisePipeline>()
            .init_resource::<SpecializedRenderPipelines<NoisePipeline>>();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum NoiseType {
    Value,
    Perlin,
    Simplex,
}

#[derive(Resource, ExtractResource, Clone, Reflect)]
#[reflect(Resource)]
pub struct NoiseSettings {
    pub ty: NoiseType,
    pub frequency: f32,
    pub amplitude: f32,
    pub enable_fbm: bool,
    pub enable_domain_warp: bool,
    pub fbm: FBMSettings,
    pub domain_warp: Vec<DomainWarpSettings>,
}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            ty: NoiseType::Value,
            frequency: 10.,
            amplitude: 0.5,
            enable_fbm: true,
            enable_domain_warp: true,
            fbm: FBMSettings {
                octaves: 6,
                lacularity: 2.,
                gain: 0.5,
            },
            domain_warp: vec![DomainWarpSettings {
                offset_a: Vec2 { x: 0., y: 0. },
                offset_b: Vec2 { x: 5.2, y: 1.3 },
            }],
        }
    }
}

#[derive(ShaderType, Clone, Copy, Reflect)]
pub struct FBMSettings {
    pub octaves: u32,
    pub lacularity: f32,
    pub gain: f32,
}

#[derive(ShaderType, Clone, Copy, Reflect)]
pub struct DomainWarpSettings {
    pub offset_a: Vec2,
    pub offset_b: Vec2,
}

#[derive(ShaderType)]
pub struct NoiseUniform {
    pub aspect: Vec2,
    pub frequency: f32,
    pub amplitude: f32,
    pub fbm: FBMSettings,
}

#[derive(Resource, Default)]
pub struct NoiseUniformBuffer {
    pub value: DynamicUniformBuffer<NoiseUniform>,
}

#[derive(Resource, Default)]
pub struct DomainWarpBuffer {
    pub value: StorageBuffer<Vec<DomainWarpSettings>>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NoisePipelineKey {
    pub ty: NoiseType,
    pub enable_fbm: bool,
    pub enable_domain_warp: bool,
}

#[derive(Resource)]
pub struct NoisePipeline {
    pub cached_id: Option<CachedRenderPipelineId>,
    pub layout: BindGroupLayout,
    pub shader: Handle<Shader>,
}

impl FromWorld for NoisePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            None,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    binding::uniform_buffer::<NoiseUniform>(false),
                    binding::storage_buffer_read_only::<Vec<DomainWarpSettings>>(false),
                ),
            ),
        );

        Self {
            shader: NOISE_SHADER,
            cached_id: None,
            layout,
        }
    }
}

impl SpecializedRenderPipeline for NoisePipeline {
    type Key = NoisePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();

        shader_defs.push(
            match key.ty {
                NoiseType::Value => "VALUE",
                NoiseType::Perlin => "PERLIN",
                NoiseType::Simplex => "SIMPLEX",
            }
            .into(),
        );

        if key.enable_fbm {
            shader_defs.push("FBM".into());
        }

        if key.enable_domain_warp {
            shader_defs.push("DOMAIN_WARP".into());
        }

        RenderPipelineDescriptor {
            label: None,
            layout: vec![self.layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
        }
    }
}

fn prepare(
    main_view_query: Query<&ExtractedView>,
    noise_settings: Res<NoiseSettings>,
    mut noise_uniform_buffer: ResMut<NoiseUniformBuffer>,
    mut domain_warp_buffer: ResMut<DomainWarpBuffer>,
    mut sp_pipelines: ResMut<SpecializedRenderPipelines<NoisePipeline>>,
    mut pipeline: ResMut<NoisePipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let main_view = main_view_query.single();

    pipeline.cached_id = Some(sp_pipelines.specialize(
        &pipeline_cache,
        &pipeline,
        NoisePipelineKey {
            ty: noise_settings.ty,
            enable_fbm: noise_settings.enable_fbm,
            enable_domain_warp: noise_settings.enable_domain_warp,
        },
    ));

    noise_uniform_buffer.value.clear();
    noise_uniform_buffer.value.push(&NoiseUniform {
        aspect: Vec2::new(
            main_view.projection.y_axis[1] / main_view.projection.x_axis[0],
            1.,
        ),
        frequency: noise_settings.frequency,
        amplitude: noise_settings.amplitude,
        fbm: noise_settings.fbm,
    });
    noise_uniform_buffer
        .value
        .write_buffer(&render_device, &render_queue);

    domain_warp_buffer
        .value
        .set(noise_settings.domain_warp.clone());
    domain_warp_buffer
        .value
        .write_buffer(&render_device, &render_queue);
}

#[derive(RenderLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoiseNodeLabel;

#[derive(Default)]
pub struct NoiseNode;

impl ViewNode for NoiseNode {
    type ViewQuery = Read<ViewTarget>;

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        view_target: QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<NoisePipeline>();
        let Some(render_pipeline) = world
            .resource::<PipelineCache>()
            .get_render_pipeline(pipeline.cached_id.unwrap())
        else {
            return Ok(());
        };

        let Some(noise_uniform) = world.resource::<NoiseUniformBuffer>().value.binding() else {
            return Ok(());
        };
        let Some(domain_warp_storage) = world.resource::<DomainWarpBuffer>().value.binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            None,
            &pipeline.layout,
            &BindGroupEntries::sequential((noise_uniform, domain_warp_storage)),
        );

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Default::default(),
            })],
            ..Default::default()
        });

        pass.set_render_pipeline(render_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);

        Ok(())
    }
}
