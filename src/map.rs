use bytemuck::{bytes_of, checked::cast_slice};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTileData {
    pub filled: u32,
    vr : f32,
    vg : f32,
    vb : f32,
    pub children: [u32; 8],
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub d: i32,
}

#[derive(Default, Debug, Clone)]
pub struct CpuTileData {
    pub filled: bool,
    pub vr : f32,
    pub vg : f32,
    pub vb : f32,
    pub children: [Option<Box<CpuTileData>>; 8],
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub d: i32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMapData {
    pub width: u32,
    pub heigth: u32,
    pub deph: u32,
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
        let mut indexes = [0_u32; 8];
        for (i, child) in self.children.iter().enumerate() {
            if let Some(child) = child.as_ref() {
                indexes[i] = child.serialize(global_index, gpu_data, indexed);
            }
        }
        gpu_data[index as usize] = GpuTileData {
            filled: if self.filled { 1 } else { 0 },
            vr : self.vr,
            vg : self.vg,
            vb : self.vb,
            children: indexes,
            x: self.x,
            y: self.y,
            z: self.z,
            d: self.d,
        };
        index
    }
}
impl MapData {
    pub fn new(d: i32) -> Self {
        Self {
            map_data: GpuMapData {
                width: 2_u32.pow(d as u32),
                heigth: 2_u32.pow(d as u32),
                deph: 2_u32.pow(d as u32),
            },
            gpu_data: Vec::new(),
            cpu_data: Box::new(CpuTileData {
                filled: false,
                vr: 0.,
                vg : 0.,
                vb : 0.,
                children: [const { Option::None }; 8],
                x: 0,
                y: 0,
                z: 0,
                d,
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
impl MapData {
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
    pub fn retrieve_value(&mut self, tar_pos: (u32, u32, u32)) -> Result<Box<CpuTileData>, ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.filled {
                return Ok(cur_tile.clone());
            }

            let w = 2_u32.pow(cur_tile.d as u32) / 2;

            let id_x = if tar_pos.0 < cur_tile.x + w { 0 } else { 1 };
            let id_y = if tar_pos.1 < cur_tile.y + w { 0 } else { 1 };
            let id_z = if tar_pos.2 < cur_tile.z + w { 0 } else { 1 };

            let id = (id_x + id_y * 2 + id_z * 4) as usize;

            if cur_tile.children[id].is_some() {
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            } else {
                break;
            }
        }
        Err(())
    }

    pub fn insert_value(&mut self, tar_pos: (u32, u32, u32), deph: i32, color : [f32; 3]) -> Result<(), ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.d == deph {
                cur_tile.vr = color[0];
                cur_tile.vg = color[1];
                cur_tile.vb = color[2];
                cur_tile.filled = true;
                return Ok(());
            }

            let w = 2_u32.pow(cur_tile.d as u32);
            let nw = w / 2;

            let id_x = if tar_pos.0 < cur_tile.x + nw { 0 } else { 1 };
            let id_y = if tar_pos.1 < cur_tile.y + nw { 0 } else { 1 };
            let id_z = if tar_pos.2 < cur_tile.z + nw { 0 } else { 1 };

            let id = (id_x + id_y * 2 + id_z * 4) as usize;

            if cur_tile.children[id].is_some() {
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            } else {
                cur_tile.children[id] = Some(Box::new(CpuTileData {
                    filled: false,
                    vr : 0.,
                    vg : 0.,
                    vb : 0.,
                    x: cur_tile.x + nw * id_x,
                    y: cur_tile.y + nw * id_y,
                    z: cur_tile.z + nw * id_z,
                    d: cur_tile.d - 1,
                    children: [const { Option::None }; 8],
                }));
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            }
        }
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
