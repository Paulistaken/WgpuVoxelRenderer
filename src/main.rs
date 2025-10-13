use std::f32;
use std::sync::Arc;
use std::time::Instant;

use rand::random_range;
use wgpu::{ShaderModuleDescriptor, ShaderSource};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};

mod cam;
mod input;
mod map;
mod screen;

fn distance(p1: &[f32; 3], p2: &[f32; 3]) -> f32 {
    f32::sqrt(
        f32::powi(p1[0] - p2[0], 2) + f32::powi(p1[1] - p2[1], 2) + f32::powi(p1[2] - p2[2], 2),
    )
}

fn min_angle_distance(a: f32, b: f32) -> f32 {
    let d1 = (a - b).abs();
    let d2 = (a + 360. - b).abs();
    let d3 = (a - 360. - b).abs();
    d1.min(d2).min(d3)
}

struct State<'a> {
    def_vir_rez: u32,
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    voxel_render_pipeline: wgpu::RenderPipeline,
    voxel_render_compute_clear_pipeline: wgpu::ComputePipeline,
    chunk_pipeline: wgpu::ComputePipeline,
    screen_data: screen::ScreenData,
    chunks_data: Vec<map::ChunkData>,
    cam_data: cam::GpuCamData,
}

struct _EguiState {
    _state: egui_winit::State,
}

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<State<'a>>,
    input: input::InputManager,
    time_log: Option<Instant>,
    delta_time: f32,
    _egui: Option<_EguiState>,
}

fn load_model_full(
    device: &wgpu::Device,
    path: &str,
    default_deph: Option<i32>,
) -> Result<map::ChunkData, Box<dyn std::error::Error>> {
    let d_d = default_deph.unwrap_or(0);
    let d_size = 2_f32.powi(d_d);
    let mut dims = [0_f32; 3];
    let mut orgin = [0_f32; 3];
    let mut n = 0;
    let vox_data = dot_vox::load(path)?;
    for model in vox_data.models.iter() {
        for voxel in model.voxels.iter() {
            dims[0] = dims[0].max(voxel.x as f32 * d_size);
            dims[1] = dims[1].max(voxel.z as f32 * d_size);
            dims[2] = dims[2].max(voxel.y as f32 * d_size);
            orgin[0] += voxel.x as f32 * d_size;
            orgin[1] += voxel.z as f32 * d_size;
            orgin[2] += voxel.y as f32 * d_size;
            n += 1;
        }
    }
    orgin[0] /= n as f32;
    orgin[1] /= n as f32;
    orgin[2] /= n as f32;
    let dim = f32::log2(dims[0].max(dims[1]).max(dims[2])).ceil() as i32;
    let mut map = map::ChunkData::new(dim);
    map.gpu_chunk_data.orgin = orgin;
    let _ = _load_model(&mut map, path, d_d);
    // map.optimize();
    map.serialize();
    map.make_buffers(device);
    Ok(map)
}
fn _load_model(
    map: &mut map::ChunkData,
    path: &str,
    deph: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let d_size = 2_f32.powi(deph);
    let vox_data = dot_vox::load(path)?;
    for model in vox_data.models.iter() {
        for voxel in model.voxels.iter() {
            let pos = (
                voxel.x as f32 * d_size,
                voxel.z as f32 * d_size,
                voxel.y as f32 * d_size,
            );
            let color = vox_data.palette.get(voxel.i as usize).unwrap();
            let color = [
                color.r as f32 / 255.,
                color.g as f32 / 255.,
                color.b as f32 / 255.,
            ];
            let _ = map.insert_value(pos, deph, color);
        }
    }
    Ok(())
}

fn gen_sphere(map: &mut map::ChunkData, middle: (f32, f32, f32), sz: f32, dp: i32) {
    let sz = sz / 2.;
    let szt = 2_f32.powi(dp);

    let range_x = ((middle.0 - sz) / szt) as u32..((middle.0 + sz) / szt) as u32;
    let range_y = ((middle.1 - sz) / szt) as u32..((middle.1 + sz) / szt) as u32;
    let range_z = ((middle.2 - sz) / szt) as u32..((middle.2 + sz) / szt) as u32;
    let mmdl = |x: f32, y: f32, z: f32| {
        f32::sqrt(
            f32::powi(x - middle.0, 2) + f32::powi(y - middle.1, 2) + f32::powi(z - middle.2, 2),
        )
    };
    for px in range_x.clone() {
        let x = (px as f32) * szt;
        for py in range_y.clone() {
            let y = (py as f32) * szt;
            for pz in range_z.clone() {
                let z = (pz as f32) * szt;
                let dst = mmdl(x, y, z);
                if dst <= sz && dst >= sz - (szt * 2.) {
                    let _ = map.insert_value(
                        (x, y, z),
                        dp,
                        [
                            random_range(0. ..1.),
                            random_range(0. ..1.),
                            random_range(0. ..1.),
                        ],
                    );
                }
            }
        }
    }
}

impl State<'_> {
    async fn new(window: Arc<Window>) -> Self {
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
        let mut chunks = vec![];
        if let Ok(mut map_data) = load_model_full(&device, "./assets/models/tree2.vox", None) {
            map_data.gpu_chunk_data.pos = [64., 64., 64.];
            chunks.push(map_data);
        }
        if let Ok(mut map_data) = load_model_full(&device, "./assets/models/cact1.vox", None) {
            map_data.gpu_chunk_data.pos = [64., 64., 64.];
            let _ = _load_model(&mut map_data, "./assets/models/cact2.vox", 0);
            map_data.gpu_chunk_data.pos = [128., 128., 32.];
            map_data.gpu_chunk_data.orgin[1] = 0.;
            map_data.serialize();
            map_data.make_buffers(&device);
            chunks.push(map_data);
        }
        if let Ok(mut map_data) = load_model_full(&device, "./assets/models/cact1.vox", None) {
            map_data.gpu_chunk_data.pos = [64., 64., 64.];
            let _ = _load_model(&mut map_data, "./assets/models/cact2.vox", 0);
            map_data.gpu_chunk_data.pos = [128., 128., 64.];
            map_data.gpu_chunk_data.orgin[1] = 0.;
            map_data.gpu_chunk_data.rot[2] = f32::consts::PI / 2.;
            map_data.serialize();
            map_data.make_buffers(&device);
            chunks.push(map_data);
        }
        for i in 0..3 {
            let mut map_data = map::ChunkData::new(7);
            gen_sphere(&mut map_data, (64., 64., 64.), 60., random_range(-3..4));
            map_data.gpu_chunk_data.pos[0] = 128. + 128. * (i as f32);
            map_data.optimize(None);
            map_data.serialize();
            map_data.make_buffers(&device);
            chunks.push(map_data);
        }
        for i in 0..3 {
            let mut map_data = map::ChunkData::new(6);
            gen_sphere(&mut map_data, (32., 32., 32.), 30., random_range(-4..2));
            map_data.gpu_chunk_data.pos[0] = 128. + 64. * (i as f32);
            map_data.gpu_chunk_data.pos[2] = 128.;
            map_data.optimize(None);
            map_data.serialize();
            map_data.make_buffers(&device);
            chunks.push(map_data);
        }
        for i in 0..3 {
            let mut map_data = map::ChunkData::new(6);
            gen_sphere(&mut map_data, (32., 32., 32.), 39., random_range(-4..2));
            map_data.gpu_chunk_data.pos[0] = 128. + 64. * (i as f32);
            map_data.gpu_chunk_data.pos[2] = 96.;
            map_data.gpu_chunk_data.pos[1] = 64.;
            map_data.optimize(None);
            map_data.serialize();
            map_data.make_buffers(&device);
            chunks.push(map_data);
        }
        {
            let mut or_pos = 0.;
            for i in (-8..=4).rev() {
                if let Ok(mut map_data) =
                    load_model_full(&device, "./assets/models/tree2.vox", Some(i))
                {
                    map_data.gpu_chunk_data.pos[2] = or_pos;
                    or_pos += map_data.gpu_chunk_data.size / 8.;
                    map_data.gpu_chunk_data.pos[0] = -128.;
                    map_data.gpu_chunk_data.pos[1] = 0.;
                    map_data.gpu_chunk_data.orgin[1] = 0.;
                    map_data.optimize(None);
                    map_data.serialize();
                    map_data.make_buffers(&device);
                    chunks.push(map_data);
                }
            }
        }
        for i in (0..=5).rev() {
            if let Ok(mut map_data) = load_model_full(&device, "./assets/models/tree2.vox", None) {
                map_data.gpu_chunk_data.pos[2] = -64.;
                map_data.gpu_chunk_data.pos[0] = 64. * (5 - i) as f32;
                map_data.gpu_chunk_data.pos[1] = 0.;
                map_data.gpu_chunk_data.orgin[1] = 0.;
                map_data.optimize(Some(i));
                map_data.serialize();
                map_data.make_buffers(&device);
                chunks.push(map_data);
            }
        }

        for i in 0..5 {
            if let Ok(mut map_data) = load_model_full(
                &device,
                "./assets/models/tree2.vox",
                Some(random_range(-1..=1)),
            ) {
                map_data.gpu_chunk_data.pos[2] = 48. * (i as f32) + random_range(-16. ..=16.);
                map_data.gpu_chunk_data.pos[0] = random_range(-32. ..=32.);
                map_data.gpu_chunk_data.pos[1] = random_range(-4. ..=4.);
                map_data.gpu_chunk_data.orgin[1] = 0.;
                chunks.push(map_data);
            }
        }
        if let Ok(mut map_data) = load_model_full(&device, "./assets/models/cact1.vox", Some(-3)) {
            map_data.gpu_chunk_data.orgin[1] = 0.;
            let _ = _load_model(&mut map_data, "./assets/models/cact2.vox", -3);
            map_data.serialize();
            map_data.make_buffers(&device);
            chunks.push(map_data);
        }

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
                    &chunks[0].get_group_layout(&device),
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
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
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

        Self {
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
        }
    }
    fn compue_chunks(&self, map_ids: &[usize]) -> Result<(), Box<dyn std::error::Error>> {
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

            for id in map_ids {
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

        self.device.poll(wgpu::MaintainBase::Wait);

        Ok(())
    }

    fn _compue_chunk(&self, map_id: usize) -> Result<(), Box<dyn std::error::Error>> {
        let screen_bind_group = self.screen_data.get_bind_group(&self.device).unwrap();

        let map_bind_group = self
            .chunks_data
            .get(map_id)
            .unwrap()
            .get_bind_group(&self.device)
            .unwrap();

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
            compute_pass.set_bind_group(1, Some(&map_bind_group), &[]);
            compute_pass.set_bind_group(2, Some(&cam_bind_group), &[]);

            compute_pass.set_pipeline(&self.chunk_pipeline);
            compute_pass.dispatch_workgroups(
                self.screen_data.gpu_data.width,
                self.screen_data.gpu_data.heigth,
                1,
            );
        }
        self.queue.submit(std::iter::once(encoder.finish()));

        self.device.poll(wgpu::MaintainBase::Wait);

        Ok(())
    }
    fn render(&self) -> Result<(), Box<dyn std::error::Error>> {
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
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        Ok(())
    }
}
impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.window = Some(Arc::clone(&window));
        let state = State::new(self.window.as_ref().unwrap().clone());
        let state = pollster::block_on(state);
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(new_size) => {
                if let Some(state) = self.state.as_mut() {
                    state.config.width = new_size.width;
                    state.config.height = new_size.height;
                    let vir_size = (
                        state.def_vir_rez,
                        (state.def_vir_rez as f32
                            * (new_size.width as f32 / new_size.height as f32))
                            as u32,
                    );
                    state.screen_data.resize(vir_size);
                    state.screen_data.set_buffers(&state.device);
                    state.cam_data.h_fov = 60.;
                    state.cam_data.v_fov =
                        state.cam_data.h_fov * (new_size.height as f32 / new_size.width as f32);
                }
            }
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let dur = match self.time_log.as_ref() {
                    Some(tl) => Instant::now()
                        .duration_since(*tl)
                        .as_millis()
                        .try_into()
                        .unwrap_or(1),
                    None => 0,
                } as f32
                    / 1000.;
                self.time_log = Some(Instant::now());

                println!("fps: {:?}", (1. / dur.max(0.001)).round());

                self.delta_time = dur / 0.16;
                self.player_input();

                if let Some(state) = self.state.as_mut() {
                    state.chunks_data.first_mut().unwrap().gpu_chunk_data.rot[2] += 0.02;
                    state.chunks_data.get_mut(1).unwrap().gpu_chunk_data.rot[1] += 0.02;
                    state.chunks_data.get_mut(2).unwrap().gpu_chunk_data.rot[1] += 0.02;
                    state.chunks_data.get_mut(3).unwrap().gpu_chunk_data.rot[1] += 0.02;
                    state.chunks_data.get_mut(3).unwrap().gpu_chunk_data.rot[2] += 0.01;
                    state.chunks_data.get_mut(3).unwrap().gpu_chunk_data.rot[0] += 0.005;

                    state.chunks_data.last_mut().unwrap().gpu_chunk_data.rot = [
                        -state.cam_data.roll.to_radians(),
                        (-state.cam_data.pitch + 70.).to_radians(),
                        (-state.cam_data.yaw - 10.).to_radians(),
                    ];
                    let cact_pos = camera_fov(8., &state.cam_data, [0., -10., -15.]);
                    state.chunks_data.last_mut().unwrap().gpu_chunk_data.pos = cact_pos;

                    let ln = state.chunks_data.len();

                    // let ids = state.chunks_data.iter().enumerate().filter_map(|(i,c)|{
                    //     if distance(&state.cam_data.pos, &c.gpu_chunk_data.pos) < c.gpu_chunk_data.size / 2.{
                    //         return Some(i);
                    //     }
                    //     let a1 = f32::atan2(-state.cam_data.pos[2] + c.gpu_chunk_data.pos[2], -state.cam_data.pos[0] + c.gpu_chunk_data.pos[0]).to_degrees();
                    //     let a2 = f32::atan2(state.cam_data.yaw.to_radians().sin(), state.cam_data.yaw.to_radians().cos()).to_degrees();
                    //     let d = min_angle_distance(a1, a2);
                    //     if d < state.cam_data.h_fov * 0.7{
                    //         Some(i)
                    //     }else{
                    //         None
                    //     }
                    // }).collect::<Vec<_>>();

                    let ids = (0..ln).collect::<Vec<_>>();

                    let _ = state.compue_chunks(&ids);

                    let _ = state.render();
                }

                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                self.input.key_event(&event);
            }
            _ => (),
        }
    }
}

fn translate_point(point: [f32; 3], chunk_data: &map::GpuChunkData) -> Option<[f32; 3]> {
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
fn camera_fov(dist: f32, cam_data: &cam::GpuCamData, dst: [f32; 3]) -> [f32; 3] {
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

impl App<'_> {
    fn player_input(&mut self) {
        if let Some(state) = self.state.as_mut() {
            if self.input.is_key_pressed(KeyCode::KeyP) {
                let npos = camera_fov(16., &state.cam_data, [0.; 3]);
                for chunk in state.chunks_data.iter_mut() {
                    if let Some(npos) =
                        translate_point([npos[0], npos[1], npos[2]], &chunk.gpu_chunk_data)
                    {
                        if npos[0] < 0. || npos[0] >= chunk.gpu_chunk_data.size {
                            continue;
                        }
                        if npos[1] < 0. || npos[1] >= chunk.gpu_chunk_data.size {
                            continue;
                        }
                        if npos[2] < 0. || npos[2] >= chunk.gpu_chunk_data.size {
                            continue;
                        }

                        let _ = chunk.insert_value((npos[0], npos[1], npos[2]), 1, [1., 1., 1.]);
                        chunk.serialize();
                        chunk.make_buffers(&state.device);
                        break;
                    }
                }
            }
            let speed = 5. * self.delta_time;
            let cam_speed = 9. * self.delta_time;

            if self.input.is_key_pressed(KeyCode::ShiftLeft) {
                state.cam_data.pos[1] -= speed;
            }
            if self.input.is_key_pressed(KeyCode::Space) {
                state.cam_data.pos[1] += speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyW) {
                let dir = f32::to_radians(state.cam_data.yaw + 0.);
                state.cam_data.pos[2] += f32::sin(dir) * speed;
                state.cam_data.pos[0] += f32::cos(dir) * speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyS) {
                let dir = f32::to_radians(state.cam_data.yaw + 180.);
                state.cam_data.pos[2] += f32::sin(dir) * speed;
                state.cam_data.pos[0] += f32::cos(dir) * speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyA) {
                let dir = f32::to_radians(state.cam_data.yaw - 90.);
                state.cam_data.pos[2] += f32::sin(dir) * speed;
                state.cam_data.pos[0] += f32::cos(dir) * speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyD) {
                let dir = f32::to_radians(state.cam_data.yaw + 90.);
                state.cam_data.pos[2] += f32::sin(dir) * speed;
                state.cam_data.pos[0] += f32::cos(dir) * speed;
            }

            if self.input.is_key_pressed(KeyCode::KeyH) {
                state.cam_data.yaw -= cam_speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyL) {
                state.cam_data.yaw += cam_speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyK) {
                state.cam_data.pitch += cam_speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyJ) {
                state.cam_data.pitch -= cam_speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyE) {
                state.cam_data.roll += cam_speed;
            }
            if self.input.is_key_pressed(KeyCode::KeyQ) {
                state.cam_data.roll -= cam_speed;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();

    let _ = event_loop.run_app(&mut app);
}
