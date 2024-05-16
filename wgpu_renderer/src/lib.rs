use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use glam::{Mat4, UVec2};
use png::ColorType;
use render::{GpuCamera, GpuMesh, Vertex};
use scene::Camera;
use wgpu::{util::*, *};
use winit::window::Window;

use crate::render::ShaderData;

mod render;
mod scene;

pub enum RendererOutput {
    Image,
    Window { window: Window },
}

pub struct WgpuRenderer<'r> {
    instance: Instance,
    surface: Option<Surface<'r>>,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    staging_belt: StagingBelt,

    pipeline: RenderPipeline,
    target: Option<&'r Texture>,
    target_view: Option<&'r TextureView>,

    dim: UVec2,

    camera: GpuCamera,
    meshes: Vec<GpuMesh>,
    dir_lights: Vec<GpuDirectionalLight>,

    camera_uniform: Buffer,

    camera_layout: BindGroupLayout,
    camera_bind_group: Option<BindGroup>,
}

impl<'r> WgpuRenderer<'r> {
    pub async fn new(output: RendererOutput, dim: UVec2, shader: ShaderSource<'r>) -> Self {
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

        let surface = {
            match output {
                RendererOutput::Image => None,
                RendererOutput::Window { window } => Some(instance.create_surface(window).unwrap()),
            }
        };

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: VertexState {
                module: &device.create_shader_module(ShaderModuleDescriptor {
                    label: None,
                    source: shader,
                }),
                entry_point: "vertex",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: None,
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let camera_uniform = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &GpuCamera::default().as_raw(),
            usage: BufferUsages::UNIFORM,
        });

        let camera_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: GpuCamera::min_binding_size(),
                },
                count: None,
            }],
        });

        log::info!("Wgpu context set up.");

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            staging_belt: StagingBelt::new(0x100),

            pipeline,
            target: None,
            target_view: None,

            dim,

            camera: GpuCamera::default(),
            meshes: Vec::new(),
            dir_lights: Vec::new(),

            camera_uniform,

            camera_layout,
            camera_bind_group: None,
        }
    }

    #[inline]
    pub fn set_render_target(&mut self, target: &Texture, target_view: &TextureView) {
        unsafe {
            self.target = Some(std::mem::transmute::<_, &'r Texture>(target));
            self.target_view = Some(std::mem::transmute::<_, &'r TextureView>(target_view));
        }
    }

    pub fn set_camera(&mut self, camera: &Camera) {
        self.camera = GpuCamera {
            view: Mat4::from_rotation_translation(camera.rotation, camera.translation),
            proj: Mat4::perspective_rh(
                camera.fov,
                self.dim.x as f32 / self.dim.y as f32,
                camera.near,
                camera.far,
            ),
        };

        self.queue
            .write_buffer(&self.camera_uniform, 0, &self.camera.as_raw());
        self.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.camera_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.camera_uniform.as_entire_binding(),
            }],
        });
    }

    pub fn draw(&self) {
        let mut command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: self.target_view.expect("No render target set."),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(command_encoder.finish()));
        log::info!("Drawn.");
    }

    pub async fn save_result(&self, path: impl AsRef<Path>) {
        let mut texture_data = Vec::<u8>::with_capacity(self.dim.element_product() as usize * 4);

        let out_staging_buffer = self.device.create_buffer(&BufferDescriptor {
            label: None,
            size: texture_data.capacity() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        command_encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &self.target.expect("No render target set"),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &out_staging_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(self.dim.x * 4 as u32),
                    rows_per_image: Some(self.dim.y as u32),
                },
            },
            Extent3d {
                width: self.dim.x,
                height: self.dim.y,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(command_encoder.finish()));
        log::info!("Copied texture");

        let buffer_slice = out_staging_buffer.slice(..);
        let (sender, receiver) = flume::bounded(1);

        buffer_slice.map_async(MapMode::Read, move |r| sender.send(r).unwrap());
        self.device.poll(Maintain::wait()).panic_on_timeout();
        receiver.recv_async().await.unwrap().unwrap();

        {
            let view = buffer_slice.get_mapped_range();
            texture_data.extend_from_slice(&view[..]);
        }

        out_staging_buffer.unmap();

        let mut png_image = Vec::with_capacity(texture_data.capacity());
        let mut encoder =
            png::Encoder::new(std::io::Cursor::new(&mut png_image), self.dim.x, self.dim.y);
        encoder.set_color(ColorType::Rgba);

        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&texture_data).unwrap();
        writer.finish().unwrap();
        log::info!("Png image encoded.");

        File::create(path).unwrap().write_all(&png_image).unwrap();
        log::info!("Render result saved.");
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
