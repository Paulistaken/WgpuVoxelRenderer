use std::{cell::Cell, default};

use bytemuck::{bytes_of, checked::cast_slice};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuScreenData {
    pub width: u32,
    pub heigth: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPixelData {
    pub val_r: f32,
    pub val_g: f32,
    pub val_b: f32,
}

#[derive(Default)]
pub struct ScreenData {
    pub gpu_data: GpuScreenData,
    pixel_data: Vec<GpuPixelData>,
    gpu_data_buffer: Option<wgpu::Buffer>,
    pixel_data_buffer: Option<wgpu::Buffer>,
}

impl ScreenData {
    pub fn new(width: u32, heigth: u32) -> Self {
        let gpu_data = GpuScreenData { width, heigth };
        let pixel_data = vec![GpuPixelData::default(); (width * heigth) as usize];
        Self {
            gpu_data,
            pixel_data,
            ..Default::default()
        }
    }
    pub fn set_buffers(&mut self, device: &wgpu::Device) {
        self.pixel_data_buffer = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("pixel_buffer"),
                contents: bytemuck::cast_slice(&self.pixel_data),
                usage: wgpu::BufferUsages::STORAGE,
            },
        ));
        self.gpu_data_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gpu_data_buffer"),
                contents: bytemuck::bytes_of(&self.gpu_data),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );
    }
    pub fn get_group_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
            ],
        })
    }
    pub fn get_bind_group(&self, device: &wgpu::Device) -> Option<wgpu::BindGroup> {
        let gpu_data_buffer = self.gpu_data_buffer.as_ref()?;
        let pixel_data_buffer = self.pixel_data_buffer.as_ref()?;
        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.get_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gpu_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pixel_data_buffer.as_entire_binding(),
                },
            ],
        }))
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTileData {
    pub filled: u32,
    pub children: [u32; 4],
    pub x: u32,
    pub y: u32,
    pub w: u32,
}

#[derive(Default, Debug, Clone)]
pub struct CpuTileData {
    pub filled: u32,
    pub children: [Option<Box<CpuTileData>>; 4],
    pub x: u32,
    pub y: u32,
    pub w: u32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMapData {
    pub width: u32,
    pub heigth: u32,
}

#[derive(Default, Debug, Clone)]
pub struct MapData {
    pub map_data: GpuMapData,
    cpu_data: Box<CpuTileData>,
    gpu_data: Vec<GpuTileData>,
    gpu_mapdata_buffer: Option<wgpu::Buffer>,
    gpu_data_buffer: Option<wgpu::Buffer>,
}
impl CpuTileData {
    fn serialize(
        &self,
        global_index: &mut u32,
        gpu_data: &mut Vec<GpuTileData>,
        indexed: &mut Vec<u32>,
    ) -> u32 {
        let index = *global_index;
        *global_index += 1;
        gpu_data.push(GpuTileData::default());
        indexed.push(index);
        let mut indexes = [0_u32; 4];
        for (i, child) in self.children.iter().enumerate() {
            if let Some(child) = child.as_ref() {
                indexes[i] = child.serialize(global_index, gpu_data, indexed);
            }
        }
        gpu_data[index as usize] = GpuTileData {
            filled: self.filled,
            children: indexes,
            x: self.x,
            y: self.y,
            w: self.w,
        };
        index
    }
}
impl MapData {
    pub fn new(w: u32) -> Self {
        Self {
            map_data: GpuMapData {
                width: w,
                heigth: w,
            },
            gpu_data: Vec::new(),
            cpu_data: Box::new(CpuTileData {
                filled: 0,
                children: [const { Option::None }; 4],
                x: 0,
                y: 0,
                w,
            }),
            ..Default::default()
        }
    }
    pub fn make_buffers(&mut self, device: &wgpu::Device) {
        self.gpu_mapdata_buffer = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytes_of(&self.map_data),
                usage: wgpu::BufferUsages::STORAGE,
            },
        ));
        self.gpu_data_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&self.gpu_data),
                usage: wgpu::BufferUsages::STORAGE,
            }),
        );
    }
}
impl MapData{
    pub fn get_group_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
        let map_data_buffer = self.gpu_mapdata_buffer.as_ref()?;
        let gpu_data_buffer = self.gpu_data_buffer.as_ref()?;
        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.get_group_layout(device),
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
impl MapData {
    pub fn retrieve_value(&mut self, tar_pos: (u32, u32)) -> Result<Box<CpuTileData>, ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.w == 1 {
                if cur_tile.x == tar_pos.0 && cur_tile.y == tar_pos.1 {
                    return Ok(cur_tile.clone());
                }
                break;
            }

            let nw = cur_tile.w / 2;

            let id_x = if tar_pos.0 < cur_tile.x + nw { 0 } else { 1 };
            let id_y = if tar_pos.1 < cur_tile.y + nw { 0 } else { 1 };

            let id = (id_x + id_y * 2) as usize;

            if cur_tile.children[id].is_some() {
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            } else {
                break;
            }
        }
        Err(())
    }

    pub fn insert_value(&mut self, val: u32, tar_pos: (u32, u32)) -> Result<(), ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.w == 1 {
                if cur_tile.x == tar_pos.0 && cur_tile.y == tar_pos.1 {
                    cur_tile.filled = val;
                    return Ok(());
                }
                break;
            }

            let nw = cur_tile.w / 2;

            let id_x = if tar_pos.0 < cur_tile.x + nw { 0 } else { 1 };
            let id_y = if tar_pos.1 < cur_tile.y + nw { 0 } else { 1 };

            let id = (id_x + id_y * 2) as usize;

            if cur_tile.children[id].is_some() {
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            } else {
                cur_tile.children[id] = Some(Box::new(CpuTileData {
                    filled: 0,
                    x: cur_tile.x + nw * id_x,
                    y: cur_tile.y + nw * id_y,
                    w: nw,
                    children: [const { Option::None }; 4],
                }));
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            }
        }
        Err(())
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
