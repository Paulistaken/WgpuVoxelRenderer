use std::sync::Arc;

use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

mod cam;
mod map;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ColorStruct {
    val: f32,
    up_d: u32,
}

struct State<'a> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    data_buffer: wgpu::Buffer,
    randval_buffer: wgpu::Buffer,
    data_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    screen_data: map::ScreenData,
    map_data: map::MapData,
    cam_data: cam::GpuCamData,
}

#[derive(Default)]
struct InputManager {
    data: std::collections::HashMap<Box<str>, bool>,
}

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<State<'a>>,
    input: InputManager,
}

impl State<'_> {
    async fn new(window: Arc<Window>) -> Self {
        let size = (800, 600);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::wgt::DeviceDescriptor {
                required_features: wgpu::Features {
                    features_wgpu: wgpu::FeaturesWGPU::default()
                        | wgpu::FeaturesWGPU::VERTEX_WRITABLE_STORAGE,
                    features_webgpu: wgpu::FeaturesWebGPU::default(),
                },
                ..Default::default()
            })
            .await
            .unwrap();
        let mut cam_data = cam::GpuCamData::default();
        cam_data.x = 1.;
        let mut screen_data = map::ScreenData::new(960, 540);
        screen_data.set_buffers(&device);
        let mut map_data = map::MapData::new(32);
        // let _ = map_data.insert_value(1, (0,0));
        // let _ = map_data.insert_value(1, (5,7));
        // let _ = map_data.insert_value(1, (9,11));
        // let _ = map_data.insert_value(1, (11,9));
        // let _ = map_data.insert_value(1, (20,20));
        // let _ = map_data.insert_value(1, (20,29));

        let _ = map_data.insert_value(1, (0, 5));
        let _ = map_data.insert_value(1, (1, 7));
        let _ = map_data.insert_value(1, (2, 5));
        let _ = map_data.insert_value(1, (4, 9));
        let _ = map_data.insert_value(1, (7, 4));
        let _ = map_data.insert_value(1, (16, 16));

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

        let to_gpu_data: Vec<ColorStruct> = vec![
            ColorStruct { val: 0., up_d: 5 },
            ColorStruct { val: 0., up_d: 40 },
        ];

        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&to_gpu_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let randval_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice::<f32, u8>(&[
                0.5, 0.35, 0.3, 0.4, 0.5, 0.6, 0.62, 0.8, 0.9, 1.0, 0.25, 0.43, 0.12, 0.05, 0.72,
                0.19, 0.81, 0.12, 0.23, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.53, 0.2, 0.5, 0.35, 0.25,
                0.43, 0.12, 0.05, 0.72, 0.19, 0.81, 0.12, 0.23,
            ]),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let bindg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/main.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[
                &bindg_layout,
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
                // 3.
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
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
            label: None,
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
            data_buffer: buffer,
            data_group_layout: bindg_layout,
            compute_pipeline,
            randval_buffer,
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
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.data_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.randval_buffer.as_entire_binding(),
                },
            ],
        });

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
            compute_pass.set_bind_group(0, Some(&bind_group), &[]);
            compute_pass.set_bind_group(1, Some(&screen_bind_group), &[]);
            compute_pass.set_bind_group(2, Some(&map_bind_group), &[]);
            compute_pass.set_bind_group(3, Some(&cam_bind_group), &[]);

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.dispatch_workgroups(self.screen_data.gpu_data.width, 1, 1);
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
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_bind_group(0, Some(&bind_group), &[]);
            render_pass.set_bind_group(1, Some(&screen_bind_group), &[]);
            render_pass.set_bind_group(2, Some(&map_bind_group), &[]);
            render_pass.set_bind_group(3, Some(&cam_bind_group), &[]);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..6, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
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
                if let Some(state) = self.state.as_mut() {
                    if self.input.data.get("w").is_some_and(|p| *p) {
                        let dir = f32::to_radians(state.cam_data.direction + 0.);
                        state.cam_data.x += f32::sin(dir) / 10.;
                        state.cam_data.y += f32::cos(dir) / 10.;
                    }
                    if self.input.data.get("s").is_some_and(|p| *p) {
                        let dir = f32::to_radians(state.cam_data.direction + 180.);
                        state.cam_data.x += f32::sin(dir) / 10.;
                        state.cam_data.y += f32::cos(dir) / 10.;
                    }
                    if self.input.data.get("a").is_some_and(|p| *p) {
                        let dir = f32::to_radians(state.cam_data.direction - 90.);
                        state.cam_data.x += f32::sin(dir) / 10.;
                        state.cam_data.y += f32::cos(dir) / 10.;
                    }
                    if self.input.data.get("d").is_some_and(|p| *p) {
                        let dir = f32::to_radians(state.cam_data.direction + 90.);
                        state.cam_data.x += f32::sin(dir) / 10.;
                        state.cam_data.y += f32::cos(dir) / 10.;
                    }
                    if self.input.data.get("q").is_some_and(|p| *p) {
                        state.cam_data.direction -= 1.;
                    }
                    if self.input.data.get("e").is_some_and(|p| *p) {
                        state.cam_data.direction += 1.;
                    }

                    let _ = state.render();
                }
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                let keyc = event
                    .text
                    .unwrap_or_default()
                    .as_str()
                    .to_string()
                    .into_boxed_str();
                match event.state {
                    winit::event::ElementState::Pressed => match self.input.data.get_mut(&keyc) {
                        Some(v) => {
                            *v = true;
                        }
                        None => {
                            self.input.data.insert(keyc, true);
                        }
                    },
                    winit::event::ElementState::Released => match self.input.data.get_mut(&keyc) {
                        Some(v) => {
                            *v = false;
                        }
                        None => {
                            self.input.data.insert(keyc, false);
                        }
                    },
                }
            }
            _ => (),
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
