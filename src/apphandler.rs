use std::f32;
use std::sync::Arc;
use std::time::Instant;

use super::input;
use super::mainstate;
use super::mainstate::State;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};

#[derive(Default)]
pub struct GameState {
    pub velocity: nalgebra::Vector3<f32>,
}
impl GameState {
    pub fn player_input<'a>(
        &mut self,
        input: &mut input::InputManager,
        state: &mut State<'a>,
        delta_time: f32,
    ) {
        state.cam_data.pos[0] += self.velocity[0];
        state.cam_data.pos[1] += self.velocity[1];
        state.cam_data.pos[2] += self.velocity[2];
        self.velocity *= 0.8;
        if (self.velocity[0].powi(2) + self.velocity[1].powi(2) + self.velocity[2].powi(2)).sqrt()
            < 0.05
        {
            self.velocity = nalgebra::Vector3::new(0., 0., 0.);
        }
        if input.is_key_pressed(KeyCode::KeyP) {
            let npos = mainstate::camera_angle_disp(&state.cam_data, [0.; 3], [0., 0., 16.]);

            for chunk in state.chunks_data.iter_mut() {
                if let Some(npos) =
                    mainstate::translate_point([npos[0], npos[1], npos[2]], &chunk.gpu_chunk_data)
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

        let speed = 5. * delta_time;
        let cam_speed = 9. * delta_time;

        {
            let mut vel_to_add = nalgebra::Vector3::new(0., 0., 0.);
            if input.is_key_pressed(KeyCode::ShiftLeft) {
                vel_to_add[1] -= 1.;
                // self.velocity[1] -= speed;
                // state.cam_data.pos[1] -= speed;
            }
            if input.is_key_pressed(KeyCode::Space) {
                vel_to_add[1] += 1.;
                // self.velocity[1] += speed;
                // state.cam_data.pos[1] += speed;
            }
            if input.is_key_pressed(KeyCode::KeyW) {
                vel_to_add += mainstate::angle_disp(&state.cam_data, [0., 0., 0.], [0., 0., 1.]);
                // self.velocity += mainstate::angle_disp(&state.cam_data, [0.,0.,0.], [0.,0.,speed]);
            }
            if input.is_key_pressed(KeyCode::KeyS) {
                vel_to_add += mainstate::angle_disp(&state.cam_data, [0., 0., 0.], [0., 0., -1.]);
                // self.velocity += mainstate::angle_disp(&state.cam_data, [0.,0.,0.], [0.,0.,-speed]);
            }
            if input.is_key_pressed(KeyCode::KeyA) {
                vel_to_add += mainstate::angle_disp(&state.cam_data, [0., 0., 0.], [1., 0., 0.]);
                // self.velocity += mainstate::angle_disp(&state.cam_data, [0.,0.,0.], [speed,0.,0.]);
            }
            if input.is_key_pressed(KeyCode::KeyD) {
                vel_to_add += mainstate::angle_disp(&state.cam_data, [0., 0., 0.], [-1., 0., 0.]);
                // self.velocity += mainstate::angle_disp(&state.cam_data, [0.,0.,0.], [-speed,0.,0.]);
            }
            let d = vel_to_add[0].powi(2) + vel_to_add[1].powi(2) + vel_to_add[2].powi(2);
            if d != 0. {
                self.velocity += (vel_to_add / d.sqrt()) * speed;
            }
        }

        if input.is_key_pressed(KeyCode::KeyH) {
            state.cam_data.yaw += cam_speed;
        }
        if input.is_key_pressed(KeyCode::KeyL) {
            state.cam_data.yaw -= cam_speed;
        }
        if input.is_key_pressed(KeyCode::KeyK) {
            state.cam_data.pitch -= cam_speed;
        }
        if input.is_key_pressed(KeyCode::KeyJ) {
            state.cam_data.pitch += cam_speed;
        }
        if input.is_key_pressed(KeyCode::KeyE) {
            state.cam_data.roll += cam_speed;
        }
        if input.is_key_pressed(KeyCode::KeyQ) {
            state.cam_data.roll -= cam_speed;
        }

        state.cam_data.yaw -= input.axis_moved_x.0 * cam_speed * 50.;
        input.axis_moved_x.0 *= 0.1;
        if input.axis_moved_x.0.abs() < 0.001 {
            input.axis_moved_x.0 = 0.;
        }
        state.cam_data.pitch += input.axis_moved_y.0 * cam_speed * 50.;
        input.axis_moved_y.0 *= 0.1;
        if input.axis_moved_y.0.abs() < 0.001 {
            input.axis_moved_y.0 = 0.;
        }
    }
}

#[derive(Default)]
pub struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<State<'a>>,
    input: input::InputManager,
    time_log: Option<Instant>,
    delta_time: f32,
    game_state: GameState,
}

impl<'a> App<'a> {
    pub fn get_state(&mut self) -> &mut State<'a> {
        self.state.as_mut().unwrap()
    }
    fn get_game_state(&mut self) -> (&mut GameState, &mut State<'a>, &mut input::InputManager) {
        let gamestate = &mut self.game_state;
        let state = self.state.as_mut().unwrap();
        let input = &mut self.input;
        (gamestate, state, input)
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
                let state = self.get_state();
                state.config.width = new_size.width * 2 / 3;
                state.config.height = new_size.height * 2 / 3;
                let vir_size = (
                    state.def_vir_rez,
                    (state.def_vir_rez as f32 * (new_size.height as f32 / new_size.width as f32))
                        as u32,
                );
                state.screen_data.resize(vir_size);
                state.screen_data.set_buffers(&state.device);
                state.cam_data.h_fov = 60.;
                state.cam_data.v_fov =
                    state.cam_data.h_fov * (new_size.height as f32 / new_size.width as f32);
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

                {
                    let delta_time = self.delta_time;
                    let (game_state, state, inp) = self.get_game_state();
                    game_state.player_input(inp, state, delta_time);
                }

                {
                    let state = self.get_state();
                    state.chunks_data.first_mut().unwrap().gpu_chunk_data.rot[2] += 0.02;
                    for i in 0..25 {
                        if i % 2 != 0 {
                            state.chunks_data.get_mut(i).unwrap().gpu_chunk_data.rot[0] += 0.02;
                        }
                        if i % 3 != 0 {
                            state.chunks_data.get_mut(i).unwrap().gpu_chunk_data.rot[1] += 0.02;
                        }
                        if i % 4 != 0 {
                            state.chunks_data.get_mut(i).unwrap().gpu_chunk_data.rot[2] += 0.02;
                        }
                    }

                    state.chunks_data.get_mut(26).unwrap().gpu_chunk_data.rot[2] += 0.02;
                    state.chunks_data.get_mut(27).unwrap().gpu_chunk_data.rot[1] += 0.02;
                    state.chunks_data.get_mut(28).unwrap().gpu_chunk_data.rot[0] += 0.01;
                    state.chunks_data.get_mut(25).unwrap().gpu_chunk_data.rot[2] += 0.005;

                    state.chunks_data.last_mut().unwrap().gpu_chunk_data.rot = [
                        -(state.cam_data.pitch + 45.).to_radians(),
                        state.cam_data.yaw.to_radians(),
                        -state.cam_data.roll.to_radians(),
                    ];
                    let cact_pos = mainstate::camera_angle_disp(
                        &state.cam_data,
                        [10., -20., 0.],
                        [0., 0., 8.],
                    );
                    state.chunks_data.last_mut().unwrap().gpu_chunk_data.pos = cact_pos;
                    state.chunks_data.last_mut().unwrap().gpu_chunk_data.orgin = [0., 0., 0.];

                    let ln = state.chunks_data.len();

                    let ids = (0..ln).collect::<Vec<_>>();

                    let _ = state.compute_chunks(&ids);

                    let _ = state.render();
                }

                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let sz = self.window.as_ref().unwrap().inner_size();
                self.window.as_ref().unwrap().set_cursor_visible(false);
                let _ = self
                    .window
                    .as_ref()
                    .unwrap()
                    .set_cursor_grab(winit::window::CursorGrabMode::Locked);

                let _ = self
                    .window
                    .as_ref()
                    .unwrap()
                    .set_cursor_position(PhysicalPosition::new(sz.width / 2, sz.height / 2));
                let _ = self
                    .window
                    .as_ref()
                    .unwrap()
                    .set_cursor_grab(winit::window::CursorGrabMode::Confined);

                let disp = (
                    position.x as f32 - (sz.width / 2) as f32,
                    position.y as f32 - (sz.height / 2) as f32,
                );
                let dd =
                    f32::sqrt((sz.width as f32 / 2.).powi(2) + (sz.height as f32 / 2.).powi(2));
                self.input.axis_moved_x.0 = disp.0 / dd;
                self.input.axis_moved_y.0 = disp.1 / dd;
                self.input.axis_moved_x.1 = std::time::Instant::now();
                self.input.axis_moved_y.1 = std::time::Instant::now();
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
