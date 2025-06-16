use std::sync::Arc;
use std::time::Instant;

use rand::random_range;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{ShaderModuleDescriptor, ShaderSource};
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};

mod cam;
mod input;
mod map;
mod screen;

struct State<'a> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    screen_data: screen::ScreenData,
    map_data: map::MapData,
    cam_data: cam::GpuCamData,
}

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<State<'a>>,
    input: input::InputManager,
    time_log: Option<Instant>,
    delta_time: f32,
}

fn make_circle(map: &mut map::MapData, middle: (f32, f32, f32), sz: f32, dp: i32) {
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
        let size = (window.inner_size().width, window.inner_size().height);
        let vir_size = (130, (130. * (size.1 as f32 / size.0 as f32)) as u32);

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
            .request_device(&wgpu::wgt::DeviceDescriptor {
                required_features: wgpu::Features {
                    features_wgpu: wgpu::FeaturesWGPU::default()
                        | wgpu::FeaturesWGPU::VERTEX_WRITABLE_STORAGE,
                    features_webgpu: wgpu::FeaturesWebGPU::default(),
                },
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await
            .unwrap();
        let mut cam_data = cam::GpuCamData::new(size);
        let mut screen_data = screen::ScreenData::new(vir_size.0, vir_size.1);
        screen_data.set_buffers(&device);
        let mut map_data = map::MapData::new(9);

        cam_data.pos[0] = 0. * 2_u32.pow(4) as f32;
        cam_data.pos[1] = 2. * 2_u32.pow(4) as f32;
        cam_data.pos[2] = 8. * 2_u32.pow(4) as f32;

        // gen_map(&mut map_data);
        make_circle(&mut map_data, (64., 64., 64.), 60., 3);
        make_circle(&mut map_data, (128., 64., 64.), 60., 2);
        make_circle(&mut map_data, (192., 64., 64.), 60., 1);

        make_circle(&mut map_data, (64., 64., 128.), 60., -2);
        make_circle(&mut map_data, (128., 64., 128.), 60., -1);
        make_circle(&mut map_data, (192., 64., 128.), 60., 0);

        make_circle(&mut map_data, (128., 128., 96.), 60., -3);

        map_data.optimize();
        map_data.serialize();
        map_data.make_buffers(&device);

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

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Main rayshader"),
            source: ShaderSource::Wgsl(include_str!("shaders/main.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[
                &screen_data.get_group_layout(&device),
                &map_data.get_group_layout(&device),
                &cam_data.get_layout(&device),
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            instance,
            config,
            adapter,
            surface,
            device,
            queue,
            render_pipeline,
            compute_pipeline,
            screen_data,
            map_data,
            cam_data,
        }
    }
    fn render(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.surface.configure(&self.device, &self.config);
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::wgt::TextureViewDescriptor::default());

        let screen_bind_group = self.screen_data.get_bind_group(&self.device).unwrap();

        let map_bind_group = self.map_data.get_bind_group(&self.device).unwrap();

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

            compute_pass.set_pipeline(&self.compute_pipeline);
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
            render_pass.set_bind_group(1, Some(&map_bind_group), &[]);
            render_pass.set_bind_group(2, Some(&cam_bind_group), &[]);

            render_pass.set_pipeline(&self.render_pipeline);
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
impl App<'_> {
    fn player_input(&mut self) {
        if let Some(state) = self.state.as_mut() {
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

            let _ = state.render();
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
