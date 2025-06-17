
struct ScreenData {
    width: u32,
    height: u32,
}
struct PixelData {
    val: vec4f,
    deph: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
}

@group(0) @binding(0) var<storage, read> screen_data: ScreenData;
@group(0) @binding(1) var<storage, read_write> pixel_data: array<PixelData>;

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
fn fs_clear(in: VertexOutput) -> @location(0) vec4<f32> {
    let idx = u32((in.vert_pos.x + 1.0) / 2.0 * f32(screen_data.width));
    let idy = u32((in.vert_pos.y + 1.0) / 2.0 * f32(screen_data.height));
    let id = screen_data.width * idy + idx;
    if pixel_data[id].deph < 0. {
        pixel_data[id].val.r = 0.1;
        pixel_data[id].val.g = 0.4;
        pixel_data[id].val.b = 0.7;
    }
    return vec4<f32>(pixel_data[id].val.r, pixel_data[id].val.g, pixel_data[id].val.b, 1.0);
}

@fragment 
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let idx = u32((in.vert_pos.x + 1.0) / 2.0 * f32(screen_data.width));
    let idy = u32((in.vert_pos.y + 1.0) / 2.0 * f32(screen_data.height));
    let id = screen_data.width * idy + idx;
    pixel_data[id].deph = -1.;
    return vec4<f32>(pixel_data[id].val.r, pixel_data[id].val.g, pixel_data[id].val.b, 1.0);
}

