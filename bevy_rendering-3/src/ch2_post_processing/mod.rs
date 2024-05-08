use bevy::{
    app::{App, Plugin, Startup},
    asset::{load_internal_asset, Assets, Handle},
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::{
        component::Component,
        query::QueryItem,
        system::{lifetimeless::Read, Commands, ResMut, Resource},
        world::{FromWorld, World},
    },
    math::primitives::Rectangle,
    render::{
        color::Color,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        globals::{GlobalsBuffer, GlobalsUniform},
        mesh::Mesh,
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FilterMode, FragmentState, MultisampleState, Operations,
            PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
            ShaderStages, ShaderType, TextureFormat, TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice},
        texture::BevyDefault,
        view::ViewTarget,
        RenderApp,
    },
    sprite::{ColorMaterial, ColorMesh2dBundle, Mesh2dHandle},
};

use bevy::render::render_resource::binding_types as binding;

const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4564131856740218563412);

pub struct Chapter2Plugin;

impl Plugin for Chapter2Plugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER_HANDLE, "monochromer.wgsl", Shader::from_wgsl);

        app.add_systems(Startup, setup).add_plugins((
            ExtractComponentPlugin::<MonochromerSettings>::default(),
            UniformComponentPlugin::<MonochromerSettings>::default(),
        ));

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .add_render_graph_node::<ViewNodeRunner<MonochromerNode>>(Core2d, MonochromerNodeLabel)
            .add_render_graph_edges(
                Core2d,
                (Node2d::MainPass, MonochromerNodeLabel, Node2d::Bloom),
            );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app.init_resource::<MonochromerPipeline>();
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(ColorMesh2dBundle {
        mesh: Mesh2dHandle(meshes.add(Rectangle::new(200., 150.))),
        material: materials.add(ColorMaterial {
            color: Color::BLUE,
            ..Default::default()
        }),
        ..Default::default()
    });
}

#[derive(Component, ExtractComponent, ShaderType, Clone)]
pub struct MonochromerSettings {
    pub speed: f32,
}

#[derive(RenderLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonochromerNodeLabel;

#[derive(Default)]
pub struct MonochromerNode;

impl ViewNode for MonochromerNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<DynamicUniformIndex<MonochromerSettings>>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, uniform_offset): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<MonochromerPipeline>();
        let Some(cached_pipeline) = world
            .resource::<PipelineCache>()
            .get_render_pipeline(pipeline.cached_id)
        else {
            return Ok(());
        };
        let settings = world.resource::<ComponentUniforms<MonochromerSettings>>();
        let globals = world.resource::<GlobalsBuffer>();

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "monochromer_bind_group",
            &pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &pipeline.sampler,
                settings.binding().unwrap(),
                globals.buffer.binding().unwrap(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("monochromer_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            ..Default::default()
        });

        render_pass.set_render_pipeline(cached_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[uniform_offset.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
pub struct MonochromerPipeline {
    cached_id: CachedRenderPipelineId,
    layout: BindGroupLayout,
    sampler: Sampler,
}

impl FromWorld for MonochromerPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "monochromer_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    binding::texture_2d(TextureSampleType::Float { filterable: true }),
                    binding::sampler(SamplerBindingType::Filtering),
                    binding::uniform_buffer::<MonochromerSettings>(true),
                    binding::uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("monochromer_sampler"),
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let cached_id =
            world
                .resource::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("monochromer_pipeline".into()),
                    layout: vec![layout.clone()],
                    push_constant_ranges: vec![],
                    vertex: fullscreen_shader_vertex_state(),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader: SHADER_HANDLE,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                });

        Self {
            cached_id,
            layout,
            sampler,
        }
    }
}
