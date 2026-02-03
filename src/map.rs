use bytemuck::{bytes_of, checked::cast_slice};
use wgpu::util::DeviceExt;

use super::vects::Vec4f;

pub mod gpu_data {
    #[repr(C)]
    #[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct GpuTileData {
        pub filled: i32,
        // pub color: [f32; 3],
        pub color: u32,
        pub children: [u32; 8],
        pub d: i32,
    }
    #[repr(C)]
    #[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct GpuChunkData {
        pub pos: [f32; 3],
        pub __fill_pos: f32,
        pub rot: [f32; 3],
        pub __fill_rot: f32,
        pub orgin: [f32; 3],
        pub max_d: i32,
        pub size: f32,
        pub __fill: [f32; 3],
    }
}

#[derive(Default, Debug, Clone)]
pub struct CpuTileData {
    pub filled: i32,
    pub color: Vec4f,
    // pub color: [u32; 3],
    pub children: [Option<Box<CpuTileData>>; 8],
    pub position: Vec4f,
    pub d: i32,
}

#[derive(Default, Debug, Clone)]
pub struct ChunkData {
    pub gpu_chunk_data: gpu_data::GpuChunkData,
    cpu_data: Box<CpuTileData>,
    gpu_data: Vec<gpu_data::GpuTileData>,
    gpu_data_buffer: Option<wgpu::Buffer>,
}
impl CpuTileData {
    fn optimize(&mut self, min_rez: Option<i32>) {
        if self.filled == self.d {
            self.children.iter_mut().for_each(|c| {
                *c = None;
            });
            return;
        }
        for c in self.children.iter_mut() {
            if let Some(c) = c.as_mut() {
                c.optimize(min_rez);
            }
        }
        if min_rez.is_some_and(|r| self.d <= r) {
            let mut n = 0;
            let mut nclr = Vec4f::ZERO;
            for c in self.children.iter() {
                if let Some(c) = c.as_ref()
                    && c.filled == c.d
                {
                    n += 1;
                    nclr += c.color;
                }
            }
            if n > 0 {
                self.color = nclr / Vec4f::from(n as f32);
                self.filled = self.d;
                self.children = [const { None }; 8];
                return;
            }
        }

        let a = !self.children.iter().any(|c| c.is_none());
        let b = !self
            .children
            .iter()
            .filter_map(|c| c.as_ref())
            .any(|c| c.filled != c.d);
        if a && b {
            let clr = self
                .children
                .iter()
                .map(|c| c.as_ref().unwrap().color)
                .fold(Vec4f::ZERO, |a, c| a + c)
                / Vec4f::from(8.);
            self.filled = self.d;
            self.color = clr;
            self.children.iter_mut().for_each(|c| {
                *c = None;
            });
        }
    }
    fn serialize(
        &self,
        global_index: &mut u32,
        gpu_data: &mut Vec<gpu_data::GpuTileData>,
        indexed: &mut Vec<u32>,
    ) -> u32 {
        let index = *global_index;
        *global_index += 1;
        gpu_data.push(gpu_data::GpuTileData::default());
        indexed.push(index);
        let mut indexes = [0_u32; 8];

        if self.filled != self.d {
            for (i, child) in self.children.iter().enumerate() {
                if let Some(child) = child.as_ref() {
                    indexes[i] = child.serialize(global_index, gpu_data, indexed);
                }
            }
        }

        let mut ncolor = 0;
        ncolor += (self.color.x() * 255.) as u32;
        ncolor += ((self.color.y() * 255.) as u32) << 8;
        ncolor += ((self.color.z() * 255.) as u32) << 16;
        gpu_data[index as usize] = gpu_data::GpuTileData {
            filled: self.filled,
            // color: [self.color.x(), self.color.y(), self.color.z()],
            color : ncolor,
            children: indexes,
            d: self.d,
        };
        index
    }
}
impl ChunkData {
    pub fn new(d: i32) -> Self {
        let size = 2_f32.powi(d);
        Self {
            gpu_chunk_data: gpu_data::GpuChunkData {
                max_d: d,
                size,
                orgin: [size / 2., size / 2., size / 2.],
                ..Default::default()
            },
            gpu_data: Vec::new(),
            cpu_data: Box::new(CpuTileData {
                filled: -1000,
                color: Vec4f::ZERO,
                children: [const { Option::None }; 8],
                position: Vec4f::ZERO,
                d,
            }),
            ..Default::default()
        }
    }
    pub fn make_buffers(&mut self, device: &wgpu::Device) {
        self.gpu_data_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&self.gpu_data),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );
    }
}
impl ChunkData {
    pub fn get_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
            ],
        })
    }
    pub fn get_bind_group(&self, device: &wgpu::Device) -> Option<wgpu::BindGroup> {
        let map_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytes_of(&self.gpu_chunk_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let gpu_data_buffer = self.gpu_data_buffer.as_ref()?;
        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &Self::get_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: map_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gpu_data_buffer.as_entire_binding(),
                },
            ],
        }))
    }
}
impl ChunkData {
    pub fn _retrieve_value(&mut self, tar_pos: (f32, f32, f32)) -> Result<Box<CpuTileData>, ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.filled == cur_tile.d {
                return Ok(cur_tile.clone());
            }

            let w = 2_f32.powi(cur_tile.d) / 2.;

            let id_x = if tar_pos.0 < cur_tile.position.x() + w {
                0
            } else {
                1
            };
            let id_y = if tar_pos.1 < cur_tile.position.y() + w {
                0
            } else {
                1
            };
            let id_z = if tar_pos.2 < cur_tile.position.z() + w {
                0
            } else {
                1
            };

            let id = (id_x + id_y * 2 + id_z * 4) as usize;

            if cur_tile.children[id].is_some() {
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            } else {
                break;
            }
        }
        Err(())
    }

    pub fn insert_value(
        &mut self,
        tar_pos: (f32, f32, f32),
        deph: i32,
        color: [f32; 3],
    ) -> Result<(), ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.d == deph {
                cur_tile.color = Vec4f::from([color[0], color[1], color[2]]);
                cur_tile.filled = cur_tile.d;
                return Ok(());
            }

            if cur_tile.filled < deph {
                cur_tile.color = Vec4f::from([color[0], color[1], color[2]]);
            }

            cur_tile.filled = cur_tile.filled.max(deph);

            let w = 2_f32.powi(cur_tile.d);
            let nw = w / 2.;

            let id_x = if tar_pos.0 < cur_tile.position.x() + nw {
                0
            } else {
                1
            };
            let id_y = if tar_pos.1 < cur_tile.position.y() + nw {
                0
            } else {
                1
            };
            let id_z = if tar_pos.2 < cur_tile.position.z() + nw {
                0
            } else {
                1
            };

            let id = (id_x + id_y * 2 + id_z * 4) as usize;

            if cur_tile.children[id].is_some() {
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            } else {
                cur_tile.children[id] = Some(Box::new(CpuTileData {
                    filled: deph,
                    color: Vec4f::from([color[0], color[1], color[2]]),
                    position: cur_tile.position
                        + Vec4f::from([id_x as f32, id_y as f32, id_z as f32]) * Vec4f::from(nw),
                    d: cur_tile.d - 1,
                    children: [const { Option::None }; 8],
                }));
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            }
        }
    }
    pub fn optimize(&mut self, min_rez: Option<i32>) {
        self.cpu_data.optimize(min_rez);
    }
    pub fn serialize(&mut self) {
        let mut tile_data = vec![];
        let mut indexes = vec![];
        let mut g_index = 0_u32;
        let _serialized = self
            .cpu_data
            .serialize(&mut g_index, &mut tile_data, &mut indexes);
        self.gpu_data = tile_data;
    }
}
