struct CamData {
    pos: vec3f,
    roll: f32,
    yaw: f32,
    pitch: f32,
    h_fov: f32,
    v_fov: f32,
}

struct ScreenData {
    width: u32,
    height: u32,
}
struct PixelData {
    val: vec4f,
    deph: f32,
}

struct ChunkData {
    pos: vec3f,
    rot: vec3f,
    orgin: vec3f,
    deph: i32,
    size: f32,
}
struct TileData {
    filled: i32,
    col : u32,
    children: array<u32,8>,
    d: i32,
}

struct node_fill_return {
    fill: u32,
    val: u32,
    b1: vec3f,
    b2: vec3f,
    size: f32,
    col: vec3f,
}

@group(0) @binding(0) var<storage, read> screen_data: ScreenData;
@group(0) @binding(1) var<storage, read_write> pixel_data: array<PixelData>;
   

@group(1) @binding(0) var<storage, read> map_data : ChunkData;
@group(1) @binding(1) var<storage, read> tiles : array<TileData>;

@group(2) @binding(0) var<storage, read> cam_data : CamData;

@compute @workgroup_size(1)fn cs_main(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    let pid = screen_data.width * id.y + id.x;

    let local_yaw = degre_to_rad(get_angle(id.x));
    let local_pitch = degre_to_rad(get_angle_pitch(id.y));

    let cam_roll = degre_to_rad(cam_data.roll);
    let cam_yaw = degre_to_rad(cam_data.yaw);
    let cam_pitch = degre_to_rad(cam_data.pitch);

    let pos = vec3f(cam_data.pos.x, cam_data.pos.y, cam_data.pos.z);

    let a_max_dist = 500.;
    let max_dist = min(a_max_dist, pixel_data[pid].deph);

    traverse_ray(pos, local_yaw, local_pitch, -cam_roll, -cam_yaw, -cam_pitch, max_dist, a_max_dist, pid);
}

fn traverse_ray(
    start_pos: vec3<f32>,
    l_yaw: f32,
    l_pit: f32,
    c_roll: f32,
    c_yaw: f32,
    c_pit: f32,
    max_dist: f32,
    a_max_dist: f32,
    pid: u32,
) {
    let l_rot_m = mat3x3f(
        cos(l_yaw), 0., sin(l_yaw),
        0., 1., 0.,
        -sin(l_yaw), 0., cos(l_yaw)
    ) * mat3x3f(
        1., 0., 0.,
        0., cos(l_pit), -sin(l_pit),
        0., sin(l_pit), cos(l_pit)
    );
    let c_rot_m = mat3x3f(
        cos(c_yaw), 0., sin(c_yaw),
        0., 1., 0.,
        -sin(c_yaw), 0., cos(c_yaw),
    ) * mat3x3f(
        1., 0., 0.,
        0., cos(c_pit), -sin(c_pit),
        0., sin(c_pit), cos(c_pit)
    ) * mat3x3f(
        cos(c_roll), -sin(c_roll), 0.,
        sin(c_roll), cos(c_roll), 0.,
        0., 0., 1
    );
    let omov = c_rot_m * (l_rot_m * vec3f(0., 0., 1.));

    let p1 = translate_point(start_pos);
    let p2 = translate_point(start_pos + omov);

    var pos = p1;

    let mov = p2 - p1;

    let e_c = enter_chunk(p1, mov, map_data.size, max_dist);

    if e_c.can == 0u {
        return;
    }

    pos = e_c.pos;

    loop {
        let dist = distance(p1, pos);

        if pos.x < 0. || pos.x > map_data.size || pos.y < 0. || pos.y > map_data.size || pos.z < 0. || pos.z > map_data.size || dist >= max_dist {
            return;
        }
        let max_deph = min(-12 + i32(dist / a_max_dist * 42.), 2);
        let d = is_node_filled(vec3(
            pos.x,
            pos.y,
            pos.z
        ), max_deph);

        switch d.fill{
            case 0u: {
                let strg = max(min(15. / dist, 1.), 0.5);
                pixel_data[pid].deph = dist;
                pixel_data[pid].val.r = strg * d.col.r;
                pixel_data[pid].val.g = strg * d.col.g;
                pixel_data[pid].val.b = strg * d.col.b;
                return;
            }
            case 2u: {
                return;
            }
            default: {
                pos = cross_area(
                    pos,
                    mov,
                    vec3(
                        d.b1.x - d.size * 0.01,
                        d.b1.y - d.size * 0.01,
                        d.b1.z - d.size * 0.01
                    ),
                    vec3(
                        d.b2.x,
                        d.b2.y,
                        d.b2.z
                    )
                );
            }
        }
    }
}

struct enterchunk {
    can: u32,
    pos: vec3f
}

fn enter_chunk(start_pos: vec3f, mov: vec3f, chunk_size: f32, max_dist: f32) -> enterchunk {
    if start_pos.x >= 0. && start_pos.x < chunk_size {
        if start_pos.y >= 0. && start_pos.y < chunk_size {
            if start_pos.z >= 0. && start_pos.z < chunk_size {
                return enterchunk(1, start_pos);
            }
        }
    }

    if (start_pos.x < 0. && mov.x < 0.) || (start_pos.y < 0. && mov.y < 0.) || (start_pos.z < 0. && mov.z < 0.) {
        return enterchunk(0, vec3f(0., 0., 0.));
    }
    if (start_pos.x > chunk_size && mov.x > 0.) || (start_pos.y > chunk_size && mov.y > 0.) || (start_pos.z > chunk_size && mov.z > 0.) {
        return enterchunk(0, vec3f(0., 0., 0.));
    }

    var t = max_dist + 1.;
    if mov.x != 0. {
        let d1 = (-start_pos.x + 0.001) / mov.x;
        let d2 = (chunk_size - start_pos.x - 0.001) / mov.x;
        if is_area_fit(start_pos, mov, chunk_size, d1) {
            t = min(t, d1);
        }
        if is_area_fit(start_pos, mov, chunk_size, d2) {
            t = min(t, d2);
        }
    }
    if mov.y != 0. {
        let d1 = (-start_pos.y + 0.001) / mov.y;
        let d2 = (chunk_size - start_pos.y - 0.001) / mov.y;
        if is_area_fit(start_pos, mov, chunk_size, d1) {
            t = min(t, d1);
        }
        if is_area_fit(start_pos, mov, chunk_size, d2) {
            t = min(t, d2);
        }
    }
    if mov.z != 0. {
        let d1 = (-start_pos.z + 0.001) / mov.z;
        let d2 = (chunk_size - start_pos.z - 0.001) / mov.z;
        if is_area_fit(start_pos, mov, chunk_size, d1) {
            t = min(t, d1);
        }
        if is_area_fit(start_pos, mov, chunk_size, d2) {
            t = min(t, d2);
        }
    }

    if t >= max_dist {
        return enterchunk(0, vec3f(0., 0., 0.));
    }

    let nmov = vec3f(mov.x * t, mov.y * t, mov.z * t);

    let npos = start_pos + nmov;

    return enterchunk(1, npos);
}

fn is_area_fit(start_pos: vec3f, mov: vec3f, chunk_size: f32, d: f32) -> bool {
    let px = start_pos.x + (mov.x * d);
    let py = start_pos.y + (mov.y * d);
    let pz = start_pos.z + (mov.z * d);
    return d >= 0f && px >= 0f && px <= chunk_size && py >= 0f && py <= chunk_size && pz >= 0f && pz <= chunk_size;
}

fn cross_area(pos: vec3<f32>, mov: vec3<f32>, b1: vec3<f32>, b2: vec3<f32>) -> vec3<f32> {
    var t = 10000.;
    var d = 0.;
    if mov.x != 0. {
        d = (b1.x - pos.x) / mov.x;
        d = abs(min(d, 10010f * sign(d)));
        t = min(t, max(d, 0.01));

        d = (b2.x - pos.x) / mov.x;
        d = abs(min(d, 10010f * sign(d)));
        t = min(t, max(d, 0.01));
    }
    if mov.y != 0. {
        d = (b1.y - pos.y) / mov.y;
        d = abs(min(d, 10010f * sign(d)));
        t = min(t, max(d, 0.01));

        d = (b2.y - pos.y) / mov.y;
        d = abs(min(d, 10010f * sign(d)));
        t = min(t, max(d, 0.01));
    }
    if mov.z != 0. {
        d = (b1.z - pos.z) / mov.z;
        d = abs(min(d, 10010f * sign(d)));
        t = min(t, max(d, 0.01));

        d = (b2.z - pos.z) / mov.z;
        d = abs(min(d, 10010f * sign(d)));
        t = min(t, max(d, 0.01));
    }

    return vec3(pos.x + mov.x * t, pos.y + mov.y * t, pos.z + mov.z * t);
}

fn is_node_filled(tar_pos: vec3f, max_deph: i32) -> node_fill_return {
    var cur_tile = tiles[0];
    var w = pow(2., f32(cur_tile.d));
    var c_pos = vec3f(0., 0., 0.);
    loop {
        if cur_tile.filled == cur_tile.d || (cur_tile.d <= max_deph && cur_tile.filled > -1000) {
            return return_fill(0u, vec3(c_pos.x, c_pos.y, c_pos.z), vec3f(f32(cur_tile.col & 255)/255,f32(cur_tile.col >> 8 & 255)/255,f32(cur_tile.col >> 16 & 255)/255), w);
        }
        let nw = w / 2.;

        let id_x: i32 = 1 - ((i32(sign(-tar_pos.x + c_pos.x + nw)) + 1) / 2);
        let id_y: i32 = 1 - ((i32(sign(-tar_pos.y + c_pos.y + nw)) + 1) / 2);
        let id_z: i32 = 1 - ((i32(sign(-tar_pos.z + c_pos.z + nw)) + 1) / 2);

        let id = id_z * 4 + id_y * 2 + id_x;

        if cur_tile.children[id] == 0u || cur_tile.d <= max_deph {
            let nx = c_pos.x + (nw * f32(id_x));
            let ny = c_pos.y + (nw * f32(id_y));
            let nz = c_pos.z + (nw * f32(id_z));
            return return_area(vec3(nx, ny, nz), nw);
        }

        cur_tile = tiles[cur_tile.children[id]];

        w /= 2.;
        c_pos.x += (nw * f32(id_x));
        c_pos.y += (nw * f32(id_y));
        c_pos.z += (nw * f32(id_z));
    }
    return return_err(2u);
}

fn return_fill(val: u32, pos: vec3f, col: vec3f, sz: f32) -> node_fill_return {
    return node_fill_return(0, val, pos, vec3(pos.x + sz, pos.y + sz, pos.z + sz), sz, col);
}
fn return_area(pos: vec3f, w: f32) -> node_fill_return {
    return node_fill_return(1, 0, pos, vec3(pos.x + w, pos.y + w, pos.z + w), w, vec3f(0., 0., 0.));
}
fn return_err(err: u32) -> node_fill_return {
    return node_fill_return(2, err, vec3(0., 0., 0.), vec3(0., 0., 0.), 0., vec3f(0., 0., 0.));
}


fn degre_to_rad(val: f32) -> f32 {
    return (val / 180.) * 3.14;
}

fn get_angle(id_x: u32) -> f32 {
    let r_fov = f32(screen_data.width / 2u);
    let step = cam_data.h_fov / 2. / r_fov;
    let x = f32(id_x) - r_fov;
    let angle = x * step;
    return angle;
}
fn get_angle_pitch(id_y: u32) -> f32 {
    let r_fov = f32(screen_data.height / 2u);
    let step = cam_data.v_fov / 2. / r_fov;
    let y = f32(id_y) - r_fov;
    let angle = y * step;
    return angle;
}

fn distance(a: vec3<f32>, b: vec3<f32>) -> f32 {
    return sqrt(pow(a.x - b.x, 2.) + pow(a.y - b.y, 2.) + pow(a.z - b.z, 2.));
}

fn translate_point(pos: vec3f) -> vec3f {
    let r_yaw = mat3x3f(
        cos(-map_data.rot.y), 0., -sin(-map_data.rot.y),
        0., 1., 0.,
        sin(-map_data.rot.y), 0., cos(-map_data.rot.y),
    );
    let r_pit = mat3x3f(
        1., 0., 0.,
        0., cos(-map_data.rot.x), -sin(-map_data.rot.x),
        0., sin(-map_data.rot.x), cos(-map_data.rot.x),
    );
    let r_roll = mat3x3f(
        cos(-map_data.rot.z), -sin(-map_data.rot.z), 0.,
        sin(-map_data.rot.z), cos(-map_data.rot.z), 0.,
        0., 0., 1.,
    );

    let rot = r_roll * r_pit * r_yaw;
    
    return (rot * (pos - map_data.pos - map_data.orgin)) + map_data.orgin;
}
