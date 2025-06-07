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
    pub val : [f32;4],
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

