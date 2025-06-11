use bytemuck::bytes_of;
use wgpu::{util::DeviceExt, wgc::device};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCamData {
    pub pos : [f32; 3],
    pub roll : f32,
    pub yaw: f32,
    pub pitch : f32,
    h_fov : f32,
    v_fov : f32,
}
impl Default for GpuCamData{
    fn default() -> Self {
        Self { pos : [0.,0.,0.], roll: 0., yaw: 0., pitch: 0., h_fov: 60., v_fov: 60. }
    }
}
impl GpuCamData {
    pub fn get_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
    pub fn get_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytes_of(self),
            usage: wgpu::BufferUsages::STORAGE,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.get_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        })
    }
}
