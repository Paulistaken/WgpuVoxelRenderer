use std::f32;
use std::sync::Arc;

use super::cam;
use super::map;
use super::screen;

use rand::random_range;
use wgpu::{ShaderModuleDescriptor, ShaderSource};
use winit::window::Window;

pub fn camera_fov(dist: f32, cam_data: &cam::GpuCamData, dst: [f32; 3]) -> [f32; 3] {
    let rot_q1 = quaternion::mul(
        quaternion::axis_angle([0., 1., 0.], -cam_data.yaw.to_radians()),
        quaternion::axis_angle([0., 0., 1.], cam_data.pitch.to_radians()),
    );
    let rot_q2 = quaternion::mul(
        quaternion::axis_angle([0., 1., 0.], dst[2].to_radians()),
        quaternion::axis_angle([0., 0., 1.], dst[1].to_radians()),
    );
    let rot_q = quaternion::mul(rot_q1, rot_q2);
    let pos = quaternion::rotate_vector(rot_q, [dist, 0., 0.]);
    [
        cam_data.pos[0] + pos[0],
        cam_data.pos[1] + pos[1],
        cam_data.pos[2] + pos[2],
    ]
}

pub fn translate_point(point: [f32; 3], chunk_data: &map::GpuChunkData) -> Option<[f32; 3]> {
    let mat_rol = ndarray::array![
        [1., 0., 0., 0.,],
        [0., chunk_data.rot[0].cos(), chunk_data.rot[0].sin(), 0.,],
        [0., -chunk_data.rot[0].sin(), chunk_data.rot[0].cos(), 0.,],
        [0., 0., 0., 1.,],
    ];
    let mat_pit = ndarray::array![
        [chunk_data.rot[2].cos(), 0., -chunk_data.rot[2].sin(), 0.],
        [0., 1., 0., 0.],
        [chunk_data.rot[2].sin(), 0., chunk_data.rot[2].cos(), 0.],
        [0., 0., 0., 1.]
    ];
    let mat_yaw = ndarray::array![
        [chunk_data.rot[1].cos(), -chunk_data.rot[1].sin(), 0., 0.],
        [chunk_data.rot[1].sin(), chunk_data.rot[1].cos(), 0., 0.,],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.]
    ];
    let mat_rot = mat_yaw.dot(&mat_pit).dot(&mat_rol);
    let mat_pos_0 = ndarray::array![
        [1., 0., 0., chunk_data.orgin[0]],
        [0., 1., 0., chunk_data.orgin[1]],
        [0., 0., 1., chunk_data.orgin[2]],
        [0., 0., 0., 1.]
    ];
    let mat_pos_1 = ndarray::array![
        [1., 0., 0., -chunk_data.pos[0]],
        [0., 1., 0., -chunk_data.pos[1]],
        [0., 0., 1., -chunk_data.pos[2]],
        [0., 0., 0., 1.]
    ];
    let mat_rot = mat_pos_0.dot(&mat_rot);
    let mat_trans = mat_rot.dot(&mat_pos_1);
    let npos = ndarray::array![point[0], point[1], point[2], 1.];
    // let npos = npos.dot(&mat_trans);
    let npos = mat_trans.dot(&npos);
    Some([
        npos.get(0).cloned().unwrap(),
        npos.get(1).cloned().unwrap(),
        npos.get(2).cloned().unwrap(),
    ])
}

pub struct State<'a> {
    pub def_vir_rez: u32,
    pub _instance: wgpu::Instance,
    pub surface: wgpu::Surface<'a>,
    pub _adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub voxel_render_pipeline: wgpu::RenderPipeline,
    pub voxel_render_compute_clear_pipeline: wgpu::ComputePipeline,
    pub chunk_pipeline: wgpu::ComputePipeline,
    pub screen_data: screen::ScreenData,
    pub chunks_data: Vec<map::ChunkData>,
    pub cam_data: cam::GpuCamData,
}

impl State<'_> {
    pub fn compute_chunks(&self, chunks_ids: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
        let screen_bind_group = self.screen_data.get_bind_group(&self.device).unwrap();

        let cam_bind_group = self.cam_data.get_bind_group(&self.device);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            compute_pass.set_bind_group(0, Some(&screen_bind_group), &[]);
            compute_pass.set_bind_group(2, Some(&cam_bind_group), &[]);

            compute_pass.set_pipeline(&self.chunk_pipeline);

            for id in chunks_ids {
                let map_data = self.chunks_data.get(*id).unwrap();
                let map_bind_group = map_data.get_bind_group(&self.device).unwrap();
                compute_pass.set_bind_group(1, Some(&map_bind_group), &[]);
                compute_pass.dispatch_workgroups(
                    self.screen_data.gpu_data.width,
                    self.screen_data.gpu_data.heigth,
                    1,
                );
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        // self.device.poll(wgpu::MaintainBase::Wait);
        Ok(())
    }

    pub fn render(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.surface.configure(&self.device, &self.config);
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let screen_bind_group = self.screen_data.get_bind_group(&self.device).unwrap();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            compute_pass.set_bind_group(0, Some(&screen_bind_group), &[]);
            compute_pass.set_pipeline(&self.voxel_render_compute_clear_pipeline);
            compute_pass.dispatch_workgroups(
                self.screen_data.gpu_data.width,
                self.screen_data.gpu_data.heigth,
                1,
            );
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.8,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_bind_group(0, Some(&screen_bind_group), &[]);

            render_pass.set_pipeline(&self.voxel_render_pipeline);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        // self.device.poll(wgpu::MaintainBase::Wait);
        Ok(())
    }
}

impl State<'_> {
    pub fn add_model(&mut self, chunk: map::ChunkData) {
        self.chunks_data.push(chunk);
    }
    pub async fn new(window: Arc<Window>) -> Self {
        let def_vir_rez = 150;
        let size = (window.inner_size().width, window.inner_size().height);
        let vir_size = (
            def_vir_rez,
            (def_vir_rez as f32 * (size.1 as f32 / size.0 as f32)) as u32,
        );

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::default()
                        | wgpu::Features::VERTEX_WRITABLE_STORAGE,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let cam_data = cam::GpuCamData::new(size);
        let mut screen_data = screen::ScreenData::new(vir_size.0, vir_size.1);
        screen_data.set_buffers(&device);
        let chunks: Vec<map::ChunkData> = vec![];

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0,
            height: size.1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let compute_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Main rayshader"),
            source: ShaderSource::Wgsl(include_str!("shaders/main.wgsl").into()),
        });
        let render_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Main rayshader"),
            source: ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[
                    &screen_data.get_group_layout(&device),
                    &map::ChunkData::get_group_layout(&device),
                    &cam_data.get_layout(&device),
                ],
                push_constant_ranges: &[],
            });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[&screen_data.get_group_layout(&device)],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::COLOR,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });
        let render_pipeline_reset =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Reset Screen Pipeline"),
                layout: Some(&render_pipeline_layout),
                module: &render_shader,
                entry_point: Some("reset_screen"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let mut state = Self {
            def_vir_rez,
            _instance: instance,
            config,
            _adapter: adapter,
            surface,
            device,
            queue,
            voxel_render_compute_clear_pipeline: render_pipeline_reset,
            voxel_render_pipeline: render_pipeline,
            chunk_pipeline: compute_pipeline,
            screen_data,
            chunks_data: chunks,
            cam_data,
        };
        state.__start();
        state
    }
}
impl State<'_> {
    pub fn __start(&mut self) {
        {
            for i in -2..=2 {
                for j in -2..=2 {
                    let mut map = map::ChunkData::new(6);
                    super::gen_sphere(&mut map, (32., 32., 32.), 64., rand::random_range(-2..4));
                    map.gpu_chunk_data.pos = [(i as f32) * 96., -64., (j as f32) * 96.];
                    map.optimize(None);
                    map.serialize();
                    map.make_buffers(&self.device);
                    self.add_model(map);
                }
            }
        }
        {
            let mut or_pos = 0.;
            for i in (-8..=4).rev() {
                if let Ok(mut map_data) =
                    super::load_model_full(&self.device, "./assets/models/tree2.vox", Some(i))
                {
                    map_data.gpu_chunk_data.pos[2] = or_pos;
                    or_pos += map_data.gpu_chunk_data.size / 8.;
                    map_data.gpu_chunk_data.pos[0] = -128.;
                    map_data.gpu_chunk_data.pos[1] = 0.;
                    map_data.gpu_chunk_data.orgin[1] = 0.;
                    map_data.optimize(None);
                    map_data.serialize();
                    map_data.make_buffers(&self.device);
                    self.add_model(map_data);
                }
            }
        }
        for i in (0..=5).rev() {
            if let Ok(mut map_data) =
                super::load_model_full(&self.device, "./assets/models/tree2.vox", None)
            {
                map_data.gpu_chunk_data.pos[2] = -64.;
                map_data.gpu_chunk_data.pos[0] = 64. * (5 - i) as f32;
                map_data.gpu_chunk_data.pos[1] = 0.;
                map_data.gpu_chunk_data.orgin[1] = 0.;
                map_data.optimize(Some(i));
                map_data.serialize();
                map_data.make_buffers(&self.device);
                self.add_model(map_data);
            }
        }

        for i in 0..5 {
            if let Ok(mut map_data) = super::load_model_full(
                &self.device,
                "./assets/models/tree2.vox",
                Some(random_range(-1..=1)),
            ) {
                map_data.gpu_chunk_data.pos[2] = 48. * (i as f32) + random_range(-16. ..=16.);
                map_data.gpu_chunk_data.pos[0] = random_range(-32. ..=32.);
                map_data.gpu_chunk_data.pos[1] = random_range(-4. ..=4.);
                map_data.gpu_chunk_data.orgin[1] = 0.;
                self.add_model(map_data);
            }
        }
        if let Ok(mut map_data) =
            super::load_model_full(&self.device, "./assets/models/cact1.vox", Some(-3))
        {
            map_data.gpu_chunk_data.orgin[1] = 0.;
            let _ = super::_load_model(&mut map_data, "./assets/models/cact2.vox", -3);
            map_data.serialize();
            map_data.make_buffers(&self.device);
            self.add_model(map_data);
        }
    }
}
