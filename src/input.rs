use wgpu::naga::FastHashMap;
use winit::{
    event::KeyEvent,
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug, Clone)]
pub struct InputManager {
    data: FastHashMap<PhysicalKey, bool>,
    data_time: FastHashMap<PhysicalKey, std::time::Instant>,
    instant_tm: std::time::Duration,
    pub axis_moved_x: (f32, std::time::Instant),
    pub axis_moved_y: (f32, std::time::Instant),
}
impl Default for InputManager {
    fn default() -> Self {
        Self {
            data: FastHashMap::default(),
            data_time: FastHashMap::default(),
            instant_tm: std::time::Duration::from_millis(10),
            axis_moved_x: (0., std::time::Instant::now()),
            axis_moved_y: (0., std::time::Instant::now()),
        }
    }
}
impl InputManager {
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        let key = PhysicalKey::Code(key);
        match self.data.get(&key) {
            None => false,
            Some(val) => *val,
        }
    }
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        let key = PhysicalKey::Code(key);
        match self.data_time.get(&key) {
            None => false,
            Some(val) => std::time::Instant::now().duration_since(*val) < self.instant_tm,
        }
    }
    pub fn key_event(&mut self, event: &KeyEvent) {
        let key = event.physical_key;
        match event.state {
            winit::event::ElementState::Pressed => {
                self.data_time.insert(key, std::time::Instant::now());
                self.data.insert(key, true);
            }
            winit::event::ElementState::Released => {
                self.data_time.remove(&key);
                self.data.insert(key, false);
            }
        }
    }
}
