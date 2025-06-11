struct CamData {
    pos : vec3f,
    roll : f32,
    yaw: f32,
    pitch: f32,
    h_fov : f32,
    v_fov : f32,
}

struct ScreenData {
    width: u32,
    height: u32,
}
struct PixelData {
    val : vec4f,
}

struct Quaternion {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

struct MapData {
    width: u32,
    heith: u32,
    deph: u32,
}
struct TileData {
    filled: u32,
    vr : f32,
    vg : f32,
    vb : f32,
    children: array<u32,8>,
    x: u32,
    y: u32,
    z: u32,
    d: u32,
}

struct node_fill_return {
    fill: u32,
    val: u32,
    b1: vec3<u32>,
    b2: vec3<u32>,
    col : vec3f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
}

@group(0) @binding(0) var<storage, read> screen_data: ScreenData;
@group(0) @binding(1) var<storage, read_write> pixel_data: array<PixelData>;
   

@group(1) @binding(0) var<storage, read> map_data : MapData;
@group(1) @binding(1) var<storage, read> tiles : array<TileData>;

@group(2) @binding(0) var<storage, read> cam_data : CamData;

@vertex  
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    var x = 0.;
    var y = 0.;

    if in_vertex_index == 0 {
        x = -1.0;
        y = -1.0;
    }
    if in_vertex_index == 1 {
        x = -1.0;
        y = 1.0;
    }
    if in_vertex_index == 2 {
        x = 1.0;
        y = -1.0;
    }
    if in_vertex_index == 3 {
        x = -1.0;
        y = 1.0;
    }
    if in_vertex_index == 4 {
        x = 1.0;
        y = -1.0;
    }
    if in_vertex_index == 5 {
        x = 1.0;
        y = 1.0;
    }

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    return out;
}
       
@fragment 
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let idx = u32((in.vert_pos.x + 1.0) / 2.0 * f32(screen_data.width));
    let idy = u32((in.vert_pos.y + 1.0) / 2.0 * f32(screen_data.height));
    let id = screen_data.width * idy + idx;
    return vec4<f32>(pixel_data[id].val.r, pixel_data[id].val.g, pixel_data[id].val.b, 1.0);
}

@compute @workgroup_size(1)fn cs_main(
    @builtin(global_invocation_id) id: vec3<u32>
) {

    let local_yaw = degre_to_rad(get_angle(id.x));
    let local_pitch = degre_to_rad(get_angle_pitch(id.y));
    
    let cam_roll = degre_to_rad(cam_data.roll);
    let cam_yaw = degre_to_rad(cam_data.yaw);
    let cam_pitch = degre_to_rad(cam_data.pitch);

    let pos = vec3f(cam_data.pos.x, cam_data.pos.y, cam_data.pos.z);

    let fill = traverse_ray(pos, local_yaw, local_pitch, cam_roll,cam_yaw, cam_pitch);

    let pid = screen_data.width * id.y + id.x;
    pixel_data[pid].val.r = 0.1;
    pixel_data[pid].val.g = 0.4;
    pixel_data[pid].val.b = 0.7;

    if fill.typ == 0 {
        let strg = min(20. / fill.dist, 1.);
        pixel_data[pid].val.r = strg * fill.col.r;
        pixel_data[pid].val.g = strg * fill.col.g;
        pixel_data[pid].val.b = strg * fill.col.b;
    }
}



struct ray_res {
    typ: u32,
    dist: f32,
    col : vec3f,
}

fn traverse_ray(
    start_pos: vec3<f32>,
    l_yaw: f32,
    l_pit: f32,
    c_roll : f32,
    c_yaw: f32,
    c_pit: f32,
) -> ray_res {
    var pos = vec3(start_pos.x, start_pos.y, start_pos.z);

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

    let mov = rotate_point(vec3f(1., 0., 0.), fin_q);

    for (var s = 0; s < 200; s += 1) {

        if i32(pos.x) < 0 || i32(pos.y) < 0 || i32(pos.z) < 0 {
            return ray_res(1, -1., vec3f(0.,0.,0.));
        }

        let d = is_node_filled(vec3(
            u32(pos.x),
            u32(pos.y),
            u32(pos.z)
        ));

        if d.fill == 0 {
            let dist = distance(start_pos, pos);
            return ray_res(0, dist, vec3f(d.col.r, d.col.g, d.col.b));
        }
        if d.fill == 1 {
            pos = cross_area(
                pos,
                mov,
                vec3(
                    f32(d.b1.x),
                    f32(d.b1.y),
                    f32(d.b1.z)
                ),
                vec3(
                    f32(d.b2.x),
                    f32(d.b2.y),
                    f32(d.b2.z)
                )
            );
        }
        if d.fill == 2 {
            break;
        }
    }
    return ray_res(2, 0., vec3f(0., 0., 0.));
}


fn cross_area(pos: vec3<f32>, mov: vec3<f32>, b1: vec3<f32>, b2: vec3<f32>) -> vec3<f32> {
    var t = 10000.;

    if mov.x > 0.0 {
        t = min(t, abs((pos.x - b2.x) / mov.x));
    } else {
        t = min(t, abs((pos.x - b1.x) / mov.x));
    }

    if mov.y > 0.0 {
        t = min(t, abs((pos.y - b2.y) / mov.y));
    } else {
        t = min(t, abs((pos.y - b1.y) / mov.y));
    }

    if mov.z > 0.0 {
        t = min(t, abs((pos.z - b2.z) / mov.z));
    } else {
        t = min(t, abs((pos.z - b1.z) / mov.z));
    }

    t += 0.1;

    return vec3(pos.x + mov.x * t, pos.y + mov.y * t, pos.z + mov.z * t);
}

fn is_node_filled(tar_pos: vec3<u32>) -> node_fill_return {
    if tar_pos.x > map_data.width {
        return return_err(1);
    }
    if tar_pos.y > map_data.heith {
        return return_err(1);
    }
    if tar_pos.y > map_data.deph {
        return return_err(1);
    }

    var cur_tile = tiles[0];

    loop {
        if cur_tile.filled == 1{
            return return_fill(0, vec3(cur_tile.x, cur_tile.y, cur_tile.z), vec3f(cur_tile.vr, cur_tile.vg, cur_tile.vb));
        }

        let w = u32(pow(2., f32(cur_tile.d)));
        let nw = w / 2;

        var id_x = 1;
        if tar_pos.x < cur_tile.x + nw {
            id_x = 0;
        }

        var id_y = 1;
        if tar_pos.y < cur_tile.y + nw {
            id_y = 0;
        }

        var id_z = 1;
        if tar_pos.z < cur_tile.z + nw {
            id_z = 0;
        }

        let id = id_z * 4 + id_y * 2 + id_x;

        if cur_tile.children[id] == 0 {
            let nx = cur_tile.x + u32(nw) * u32(id_x);
            let ny = cur_tile.y + u32(nw) * u32(id_y);
            let nz = cur_tile.z + u32(nw) * u32(id_z);
            return return_area(vec3(nx, ny, nz), nw);
        }

        cur_tile = tiles[cur_tile.children[id]];
    }
    return return_err(2);
}

fn return_fill(val: u32, pos: vec3<u32>, col : vec3f) -> node_fill_return {
    return node_fill_return(0, val, pos, vec3(pos.x + 1, pos.y + 1, pos.z + 1), col);
}
fn return_area(pos: vec3<u32>, w: u32) -> node_fill_return {
    return node_fill_return(1, 0, pos, vec3(pos.x + w, pos.y + w, pos.z + w), vec3f(0., 0., 0.));
}
fn return_err(err: u32) -> node_fill_return {
    return node_fill_return(2, err, vec3(0, 0, 0), vec3(0, 0, 0), vec3f(0., 0., 0.));
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
    return sqrt(pow(a.x - b.x, 2) + pow(a.y - b.y, 2) + pow(a.z - b.z, 2));
}
