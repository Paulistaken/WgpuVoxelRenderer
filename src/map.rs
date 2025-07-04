use bytemuck::{bytes_of, checked::cast_slice};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTileData {
    pub filled: u32,
    vr: f32,
    vg: f32,
    vb: f32,
    pub children: [u32; 8],
    pub d: i32,
}

#[derive(Default, Debug, Clone)]
pub struct CpuTileData {
    pub filled: bool,
    pub vr: f32,
    pub vg: f32,
    pub vb: f32,
    pub children: [Option<Box<CpuTileData>>; 8],
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub d: i32,
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuChunkData {
    pub max_d: i32,
    pub size: f32,
    __filla : f32,
    __fillb : f32,
    pub pos : [f32; 3],
    __fill1 : f32,
    pub rot : [f32; 3],
    __fill2 : f32,
    pub orgin : [f32; 3],
    __fill3 : f32,
}

#[derive(Default, Debug, Clone)]
pub struct ChunkData {
    pub gpu_chunk_data: GpuChunkData,
    cpu_data: Box<CpuTileData>,
    gpu_data: Vec<GpuTileData>,
    gpu_data_buffer: Option<wgpu::Buffer>,
}
impl CpuTileData {
    fn optimize(&mut self, min_rez : Option<i32>) {
        if self.filled {
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
        if min_rez.is_some_and(|r| self.d <= r){
            let mut n = 0;
            let mut nclr = [0_f32; 3];
            for c in self.children.iter(){
                if let Some(c) = c.as_ref(){
                    if c.filled{
                        n += 1;
                        nclr[0] += c.vr;
                        nclr[1] += c.vg;
                        nclr[2] += c.vb;
                    }
                }
            }
            if n > 0{
                self.vr = nclr[0] / n as f32;
                self.vg = nclr[1] / n as f32;
                self.vb = nclr[2] / n as f32;
                self.filled = true;
                self.children = [const { None };8];
                return;
            }
        }

        let a = !self.children.iter().any(|c| c.is_none());
        let b = !self
            .children
            .iter()
            .filter_map(|c| c.as_ref())
            .any(|c| !c.filled);
        if a && b {
            let clr = self
                .children
                .iter()
                .map(|c| c.as_ref().unwrap())
                .fold([0., 0., 0.], |a, c| [a[0] + c.vr, a[1] + c.vg, a[2] + c.vb]);
            self.filled = true;
            self.vr = clr[0] / 8.;
            self.vg = clr[1] / 8.;
            self.vb = clr[2] / 8.;
            self.children.iter_mut().for_each(|c| {
                *c = None;
            });
        }
    }
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

        if !self.filled {
            for (i, child) in self.children.iter().enumerate() {
                if let Some(child) = child.as_ref() {
                    indexes[i] = child.serialize(global_index, gpu_data, indexed);
                }
            }
        }

        gpu_data[index as usize] = GpuTileData {
            filled: if self.filled { 1 } else { 0 },
            vr: self.vr,
            vg: self.vg,
            vb: self.vb,
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
            gpu_chunk_data: GpuChunkData {
                max_d: d,
                size,
                orgin : [size / 2., size / 2., size / 2.],
                ..Default::default()
            },
            gpu_data: Vec::new(),
            cpu_data: Box::new(CpuTileData {
                filled: false,
                vr: 0.,
                vg: 0.,
                vb: 0.,
                children: [const { Option::None }; 8],
                x: 0.,
                y: 0.,
                z: 0.,
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
        let map_data_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytes_of(&self.gpu_chunk_data),
                usage: wgpu::BufferUsages::STORAGE,
            },
        );
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
impl ChunkData {
    pub fn _retrieve_value(&mut self, tar_pos: (f32, f32, f32)) -> Result<Box<CpuTileData>, ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.filled {
                return Ok(cur_tile.clone());
            }

            let w = 2_f32.powi(cur_tile.d) / 2.;

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

    pub fn insert_value(
        &mut self,
        tar_pos: (f32, f32, f32),
        deph: i32,
        color: [f32; 3],
    ) -> Result<(), ()> {
        let mut cur_tile = &mut self.cpu_data;
        loop {
            if cur_tile.d == deph {
                cur_tile.vr = color[0];
                cur_tile.vg = color[1];
                cur_tile.vb = color[2];
                cur_tile.filled = true;
                return Ok(());
            }

            let w = 2_f32.powi(cur_tile.d);
            let nw = w / 2.;

            let id_x = if tar_pos.0 < cur_tile.x + nw { 0 } else { 1 };
            let id_y = if tar_pos.1 < cur_tile.y + nw { 0 } else { 1 };
            let id_z = if tar_pos.2 < cur_tile.z + nw { 0 } else { 1 };

            let id = (id_x + id_y * 2 + id_z * 4) as usize;

            if cur_tile.children[id].is_some() {
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            } else {
                cur_tile.children[id] = Some(Box::new(CpuTileData {
                    filled: false,
                    vr: 0.,
                    vg: 0.,
                    vb: 0.,
                    x: cur_tile.x + nw * id_x as f32,
                    y: cur_tile.y + nw * id_y as f32,
                    z: cur_tile.z + nw * id_z as f32,
                    d: cur_tile.d - 1,
                    children: [const { Option::None }; 8],
                }));
                cur_tile = cur_tile.children[id].as_mut().unwrap();
            }
        }
    }
    pub fn optimize(&mut self, min_rez : Option<i32>) {
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
