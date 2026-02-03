use std::f32;

use rand::random_range;
use winit::event_loop::{ControlFlow, EventLoop};

mod apphandler;
mod cam;
mod input;
mod mainstate;
mod map;
mod screen;
mod voxelize;
mod vects;

pub fn load_model_full(
    device: &wgpu::Device,
    path: &str,
    default_deph: Option<i32>,
) -> Result<map::ChunkData, Box<dyn std::error::Error>> {
    let d_d = default_deph.unwrap_or(0);
    let d_size = 2_f32.powi(d_d);
    let mut dims = [0_f32; 3];
    // let mut orgin = [0_f32; 3];
    let mut orgin = wide::f32x4::splat(0.0);
    let mut n = 0;
    let vox_data = dot_vox::load(path)?;
    for model in vox_data.models.iter() {
        for voxel in model.voxels.iter() {
            dims[0] = dims[0].max(voxel.x as f32 * d_size);
            dims[1] = dims[1].max(voxel.z as f32 * d_size);
            dims[2] = dims[2].max(voxel.y as f32 * d_size);
            orgin += wide::f32x4::from([voxel.x as f32 * d_size,voxel.y as f32 * d_size,voxel.z as f32 * d_size,0.]);
            n += 1;
        }
    }
    orgin /= wide::f32x4::splat(n as f32);
    let dim = f32::log2(dims[0].max(dims[1]).max(dims[2])).ceil() as i32;
    let mut map = map::ChunkData::new(dim);
    map.gpu_chunk_data.orgin = [orgin.as_array()[0],orgin.as_array()[1],orgin.as_array()[2]];
    let _ = _load_model(&mut map, path, d_d);
    // map.optimize();
    map.serialize();
    map.make_buffers(device);
    Ok(map)
}
pub fn _load_model(
    map: &mut map::ChunkData,
    path: &str,
    deph: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let d_size = 2_f32.powi(deph);
    let vox_data = dot_vox::load(path)?;
    for model in vox_data.models.iter() {
        for voxel in model.voxels.iter() {
            let pos = (
                voxel.x as f32 * d_size,
                voxel.z as f32 * d_size,
                voxel.y as f32 * d_size,
            );
            let color = vox_data.palette.get(voxel.i as usize).unwrap();
            let color = [
                color.r as f32 / 255.,
                color.g as f32 / 255.,
                color.b as f32 / 255.,
            ];
            let _ = map.insert_value(pos, deph, color);
        }
    }
    Ok(())
}

pub fn gen_sphere(map: &mut map::ChunkData, middle: (f32, f32, f32), sz: f32, dp: i32) {
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

#[tokio::main]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = apphandler::App::default();

    let _ = event_loop.run_app(&mut app);
}
