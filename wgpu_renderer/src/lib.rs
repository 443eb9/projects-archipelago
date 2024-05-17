use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use buffer::StorageBuffer;
use glam::{Mat4, UVec2};
use png::ColorType;
use render::{GpuCamera, GpuDirectionalLight, GpuMesh, Vertex};
use scene::{Camera, DirectionalLight};
use wgpu::{util::*, *};

use crate::render::ShaderData;

pub mod buffer;
pub mod render;
pub mod scene;

pub struct WgpuImageRenderer {
    internal: WgpuRenderer,
    target: Texture,
    target_view: TextureView,
}

impl WgpuImageRenderer {
    pub async fn new(
        dim: UVec2,
        shader: ShaderSource<'_>,
        renderer_config: Option<RendererConfig>,
    ) -> Self {
        let renderer = WgpuRenderer::new(shader, renderer_config).await;

        let target = renderer.device().create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: dim.x,
                height: dim.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });
        let target_view = target.create_view(&TextureViewDescriptor::default());

        Self {
            internal: renderer,
            target,
            target_view,
        }
    }

    #[inline]
    pub fn renderer(&self) -> &WgpuRenderer {
        &self.internal
    }

    #[inline]
    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer {
        &mut self.internal
    }

    pub async fn draw(&mut self) {
        self.internal.draw(&self.target_view);
    }

    pub async fn save_result(&self, path: impl AsRef<Path>) {
        let extent = self.target.size();
        let mut texture_data =
            Vec::<u8>::with_capacity((extent.width * extent.height * 4) as usize);

        let out_staging_buffer = self.internal.device.create_buffer(&BufferDescriptor {
            label: None,
            size: texture_data.capacity() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut command_encoder = self
            .internal
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        command_encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &self.target,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &out_staging_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(extent.width * 4 as u32),
                    rows_per_image: Some(extent.height as u32),
                },
            },
            extent,
        );
        self.internal.queue.submit(Some(command_encoder.finish()));
        log::info!("Copied texture");

        let buffer_slice = out_staging_buffer.slice(..);
        let (sender, receiver) = flume::bounded(1);

        buffer_slice.map_async(MapMode::Read, move |r| sender.send(r).unwrap());
        self.internal
            .device
            .poll(Maintain::wait())
            .panic_on_timeout();
        receiver.recv_async().await.unwrap().unwrap();

        {
            let view = buffer_slice.get_mapped_range();
            texture_data.extend_from_slice(&view[..]);
        }

        out_staging_buffer.unmap();

        let mut png_image = Vec::with_capacity(texture_data.capacity());
        let mut encoder = png::Encoder::new(
            std::io::Cursor::new(&mut png_image),
            extent.width,
            extent.height,
        );
        encoder.set_color(ColorType::Rgba);

        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&texture_data).unwrap();
        writer.finish().unwrap();
        log::info!("Png image encoded.");

        File::create(path).unwrap().write_all(&png_image).unwrap();
        log::info!("Render result saved.");
    }
}

pub struct WgpuSurfaceRenderer<'r> {
    internal: WgpuRenderer,
    surface: Surface<'r>,
}

impl<'r> WgpuSurfaceRenderer<'r> {
    pub async fn new(
        target: impl Into<SurfaceTarget<'r>>,
        dim: UVec2,
        shader: ShaderSource<'_>,
        renderer_config: Option<RendererConfig>,
    ) -> Self {
        let renderer = WgpuRenderer::new(shader, renderer_config).await;
        let surface = renderer.instance.create_surface(target).unwrap();

        let sr = Self {
            internal: renderer,
            surface,
        };
        sr.resize(dim);
        sr
    }

    pub fn resize(&self, dim: UVec2) {
        self.surface.configure(
            &self.internal.device,
            &SurfaceConfiguration {
                present_mode: PresentMode::AutoVsync,
                ..self
                    .surface
                    .get_default_config(&self.internal.adapter, dim.x, dim.y)
                    .unwrap()
            },
        );
    }

    pub fn draw(&self) {
        let Ok(frame) = self.surface.get_current_texture() else {
            log::error!("Failed to acquire next swap chain texture.");
            return;
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        self.internal.draw(&view);
        frame.present();
    }

    #[inline]
    pub fn renderer(&self) -> &WgpuRenderer {
        &self.internal
    }

    #[inline]
    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer {
        &mut self.internal
    }
}

pub struct RendererConfig {
    pub primary_target_format: TextureFormat,
    pub clear_color: Color,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            primary_target_format: TextureFormat::Rgba8Unorm,
            clear_color: Color::TRANSPARENT,
        }
    }
}

pub struct WgpuRenderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    config: RendererConfig,

    pipeline: RenderPipeline,

    camera: GpuCamera,
    pub meshes: Vec<GpuMesh>,
    pub dir_lights: Vec<DirectionalLight>,

    camera_uniform: Option<Buffer>,
    dir_lights_storage: StorageBuffer<GpuDirectionalLight>,

    scene_layout: BindGroupLayout,
    scene_bind_group: Option<BindGroup>,
}

impl WgpuRenderer {
    pub async fn new(shader: ShaderSource<'_>, config: Option<RendererConfig>) -> Self {
        let config = config.unwrap_or_default();

        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .init();

        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .unwrap();

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: shader,
        });

        let scene_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: GpuCamera::min_binding_size(),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&scene_layout],
                ..Default::default()
            })),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vertex",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: config.primary_target_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        log::info!("Wgpu context set up.");

        Self {
            instance,
            adapter,
            device,
            queue,
            config,

            pipeline,

            camera: GpuCamera::default(),
            meshes: Vec::new(),
            dir_lights: Vec::new(),

            camera_uniform: None,
            dir_lights_storage: StorageBuffer::default(),

            scene_layout,
            scene_bind_group: None,
        }
    }

    pub fn set_camera(&mut self, camera: &Camera) {
        self.camera = GpuCamera {
            view: camera.transform.compute_matrix(),
            proj: Mat4::perspective_rh(camera.fov, camera.aspect_ratio, camera.near, camera.far),
        };

        self.camera_uniform = Some(self.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &self.camera.as_raw(),
            usage: BufferUsages::UNIFORM,
        }));
    }

    pub fn write_scene(&mut self) {
        self.dir_lights.iter().for_each(|l| {
            self.dir_lights_storage.push(&GpuDirectionalLight {
                translation: l.translation,
                direction: l.direction,
                color: l.color,
            });
        });
        self.dir_lights_storage.write(&self.device, &self.queue);

        let Some(dir_lights) = self.dir_lights_storage.binding() else {
            log::error!("Failed to get bindng resource for directional lights.");
            return;
        };

        self.scene_bind_group = Some(self.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.scene_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.camera_uniform.as_ref().unwrap().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: dir_lights,
                },
            ],
        }));
    }

    pub fn draw(&self, target: &TextureView) {
        let Some(scene) = &self.scene_bind_group else {
            log::error!("Failed to get bind group for scene.");
            return;
        };

        let mut command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.config.clear_color),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, scene, &[]);

            for mesh in &self.meshes {
                pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                pass.draw(0..mesh.vertex_count, 0..1);
            }
        }

        self.queue.submit(Some(command_encoder.finish()));
    }

    pub fn load_obj(&mut self, path: impl AsRef<Path>) {
        let mut source = Vec::new();
        File::open(path).unwrap().read_to_end(&mut source).unwrap();
        let obj = obj::ObjData::load_buf(&source[..]).unwrap();

        let mut vertices = Vec::new();
        for object in obj.objects {
            for group in object.groups {
                vertices.clear();
                for poly in group.polys {
                    for end_index in 2..poly.0.len() {
                        for &index in &[0, end_index - 1, end_index] {
                            let obj::IndexTuple(position_id, Some(_texture_id), Some(normal_id)) =
                                poly.0[index]
                            else {
                                unreachable!()
                            };

                            vertices.push(Vertex {
                                position: obj.position[position_id].into(),
                                normal: obj.normal[normal_id].into(),
                            });
                        }
                    }
                }

                let vertex_buf = self.device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertices),
                    usage: BufferUsages::VERTEX,
                });
                self.meshes.push(GpuMesh {
                    vertex_count: vertices.len() as u32,
                    vertex_buf,
                });
            }
        }
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }
}
