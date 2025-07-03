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

struct Quaternion {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

struct ChunkData {
    deph: i32,
    size: f32,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    pitch: f32,
    roll: f32,
}
struct TileData {
    filled: u32,
    vr: f32,
    vg: f32,
    vb: f32,
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

    var max_dist = 3000.;
    if pixel_data[pid].deph > 0. {
        max_dist = min(max_dist, pixel_data[pid].deph);
    }

    let fill = traverse_ray(pos, local_yaw, local_pitch, cam_roll, cam_yaw, cam_pitch, max_dist);

    if fill.typ == 0 {
        let strg = min(15. / fill.dist, 1.);
        pixel_data[pid].deph = fill.dist;
        pixel_data[pid].val.r = strg * fill.col.r;
        pixel_data[pid].val.g = strg * fill.col.g;
        pixel_data[pid].val.b = strg * fill.col.b;
    }
}



struct ray_res {
    typ: u32,
    dist: f32,
    col: vec3f,
}

fn traverse_ray(
    start_pos: vec3<f32>,
    l_yaw: f32,
    l_pit: f32,
    c_roll: f32,
    c_yaw: f32,
    c_pit: f32,
    max_dist: f32,
) -> ray_res {
    let fin_q = quaternion_multiply(
        quaternion_multiply(
            rotate_q_by_axis(-c_yaw, vec3f(0., 1., 0.)),
            quaternion_multiply(
                rotate_q_by_axis(c_pit, vec3f(0., 0., 1.)),
                rotate_q_by_axis(c_roll, vec3f(1., 0., 0.))
            )
        ),
        quaternion_multiply(
            rotate_q_by_axis(l_pit, vec3f(0., 0., 1.)),
            rotate_q_by_axis(-l_yaw, vec3f(0., 1., 0.)),
        ),
    );

    let op1 = start_pos;
    let omov = rotate_point(vec3f(1., 0., 0.), fin_q);
    let op2 = op1 + omov;

    let p1 = translate_point(op1);
    let p2 = translate_point(op2);

    var pos = p1;

    let mov = p2 - p1;

    let e_c = enter_chunk(p1, mov, map_data.size, max_dist);

    if e_c.can == 0 {
        return ray_res(2, 0., vec3f(0., 0., 0.));
    }

    pos = e_c.pos;

    for (; ;) {

        if distance(p1, pos) > max_dist {
            break;
        }

        let d = is_node_filled(vec3(
            pos.x,
            pos.y,
            pos.z
        ));

        if d.fill == 0 {
            let dist = distance(p1, pos);
            return ray_res(0, dist, vec3f(d.col.r, d.col.g, d.col.b));
        }
        if d.fill == 1 {
            pos = cross_area(
                pos,
                mov,
                vec3(
                    d.b1.x - d.size / 90.,
                    d.b1.y - d.size / 90.,
                    d.b1.z - d.size / 90.
                ),
                vec3(
                    d.b2.x,
                    d.b2.y,
                    d.b2.z
                )
            );
        }
        if d.fill == 2 {
            break;
        }
    }
    return ray_res(2, 0., vec3f(0., 0., 0.));
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

    var t = max_dist + 10.;
    if mov.x != 0 {
        let d1 = (-start_pos.x + 0.001) / mov.x;
        let d2 = (chunk_size - start_pos.x - 0.001) / mov.x;
        if is_area_fit(start_pos, mov, chunk_size, d1) {
            t = min(t, d1);
        }
        if is_area_fit(start_pos, mov, chunk_size, d2) {
            t = min(t, d2);
        }
    }
    if mov.y != 0 {
        let d1 = (-start_pos.y + 0.001) / mov.y;
        let d2 = (chunk_size - start_pos.y - 0.001) / mov.y;
        if is_area_fit(start_pos, mov, chunk_size, d1) {
            t = min(t, d1);
        }
        if is_area_fit(start_pos, mov, chunk_size, d2) {
            t = min(t, d2);
        }
    }
    if mov.z != 0 {
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
    if d >= 0. {
        let px = start_pos.x + (mov.x * d);
        let py = start_pos.y + (mov.y * d);
        let pz = start_pos.z + (mov.z * d);
        if px >= 0. && px <= chunk_size {
            if py >= 0. && py <= chunk_size {
                if pz >= 0. && pz <= chunk_size {
                    return true;
                }
            }
        }
    }
    return false;
}

fn cross_area(pos: vec3<f32>, mov: vec3<f32>, b1: vec3<f32>, b2: vec3<f32>) -> vec3<f32> {
    var t = 10000.;
    var d = 0.;
    if mov.x != 0. {
        d = (b1.x - pos.x) / mov.x;
        if d > 0. {
            t = min(t, d);
        }
        d = (b2.x - pos.x) / mov.x;
        if d > 0. {
            t = min(t, d);
        }
    }
    if mov.y != 0. {
        d = (b1.y - pos.y) / mov.y;
        if d > 0. {
            t = min(t, d);
        }
        d = (b2.y - pos.y) / mov.y;
        if d > 0. {
            t = min(t, d);
        }
    }
    if mov.z != 0. {
        d = (b1.z - pos.z) / mov.z;
        if d > 0. {
            t = min(t, d);
        }
        d = (b2.z - pos.z) / mov.z;
        if d > 0. {
            t = min(t, d);
        }
    }

    return vec3(pos.x + mov.x * t, pos.y + mov.y * t, pos.z + mov.z * t);
}

fn is_node_filled(tar_pos: vec3f) -> node_fill_return {
    if tar_pos.x < 0. || tar_pos.x > map_data.size {
        return return_err(1);
    }
    if tar_pos.y < 0. || tar_pos.y > map_data.size {
        return return_err(1);
    }
    if tar_pos.z < 0. || tar_pos.z > map_data.size {
        return return_err(1);
    }

    var cur_tile = tiles[0];
    var w = pow(2., f32(cur_tile.d));
    var c_pos = vec3f(0., 0., 0.);
    loop {
        if cur_tile.filled == 1 {
            return return_fill(0, vec3(c_pos.x, c_pos.y, c_pos.z), vec3f(cur_tile.vr, cur_tile.vg, cur_tile.vb), w);
        }
        let nw = w / 2.;

        var id_x = 1;
        if tar_pos.x < c_pos.x + nw {
            id_x = 0;
        }

        var id_y = 1;
        if tar_pos.y < c_pos.y + nw {
            id_y = 0;
        }

        var id_z = 1;
        if tar_pos.z < c_pos.z + nw {
            id_z = 0;
        }

        let id = id_z * 4 + id_y * 2 + id_x;

        if cur_tile.children[id] == 0 {
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
    return return_err(2);
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

fn rotate_point(point: vec3<f32>, rotation: Quaternion) -> vec3<f32> {
    let q_point = Quaternion(point.x, point.y, point.z, 0.0);
    let q_conj = Quaternion(-rotation.x, -rotation.y, -rotation.z, rotation.w);

    let rotated_q = quaternion_multiply(quaternion_multiply(rotation, q_point), q_conj);

    let rotated_point = vec3<f32>(rotated_q.x, rotated_q.y, rotated_q.z);
    return rotated_point;
}
fn rotate_q_by_axis(a: f32, ax: vec3<f32>) -> Quaternion {
    return Quaternion(
        ax.x * sin(a / 2.),
        ax.y * sin(a / 2.),
        ax.z * sin(a / 2.),
        cos(a / 2.),
    );
}
fn quaternion_multiply(q1: Quaternion, q2: Quaternion) -> Quaternion {
    let q = Quaternion(
        q1.w * q2.x + q1.x * q2.w + q1.y * q2.z - q1.z * q2.y,
        q1.w * q2.y - q1.x * q2.z + q1.y * q2.w + q1.z * q2.x,
        q1.w * q2.z + q1.x * q2.y - q1.y * q2.x + q1.z * q2.w,
        q1.w * q2.w - q1.x * q2.x - q1.y * q2.y - q1.z * q2.z,
    );
    return q;
}

fn degre_to_rad(val: f32) -> f32 {
    return (val / 180.) * 3.14;
}

fn get_angle(id_x: u32) -> f32 {
    let r_fov = f32(screen_data.width / 2);
    let step = cam_data.h_fov / 2. / r_fov;
    let x = f32(id_x) - r_fov;
    let angle = x * step;
    return angle;
}
fn get_angle_pitch(id_y: u32) -> f32 {
    let r_fov = f32(screen_data.height / 2);
    let step = cam_data.v_fov / 2. / r_fov;
    let y = f32(id_y) - r_fov;
    let angle = y * step;
    return angle;
}

fn distance(a: vec3<f32>, b: vec3<f32>) -> f32 {
    return sqrt(pow(a.x - b.x, 2.) + pow(a.y - b.y, 2.) + pow(a.z - b.z, 2.));
}

fn translate_point(pos: vec3f) -> vec3f {
    let mat_pit = mat4x4(
        cos(map_data.yaw), 0., -sin(map_data.yaw), 0.,
        0., 1., 0., 0.,
        sin(map_data.yaw), 0., cos(map_data.yaw), 0.,
        0., 0., 0., 1.,
    );
    let mat_yaw = mat4x4(
        cos(map_data.pitch), -sin(map_data.pitch), 0., 0.,
        sin(map_data.pitch), cos(map_data.pitch), 0., 0.,
        0., 0., 1., 0,
        0., 0., 0., 1.,
    );
    let mat_rot = mat_pit * mat_yaw;
    let mat_pos_0 = mat4x4f(
        1., 0., 0., map_data.size/2.,
        0., 1., 0., map_data.size/2.,
        0., 0., 1., map_data.size/2.,
        0., 0., 0., 1.,
    );
    let mat_pos = mat4x4f(
        1., 0., 0., -map_data.x,
        0., 1., 0., -map_data.y,
        0., 0., 1., -map_data.z,
        0., 0., 0., 1.,
    );
    let mat_rot_2 = mat_rot * mat_pos_0;
    let mat_trans = mat_pos * mat_rot_2;

    let npos = vec4f(pos.x, pos.y, pos.z, 1.) * mat_trans;

    return vec3f(npos.x, npos.y, npos.z);
}
