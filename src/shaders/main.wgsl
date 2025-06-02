//wtf  

struct CamData{
    x : f32,
    y : f32,
    dir : f32,
}

struct ScreenData{
    width: u32,
    height: u32,
}
struct PixelData{
    val_r : f32,
    val_g : f32,
    val_b : f32,
}

struct ColorData{
    val : f32,
    up_d : u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
}

@group(0) @binding(0) var<storage, read_write> clr: array<ColorData>;
@group(0) @binding(1) var<storage, read> rand_val: array<f32>;

@group(1) @binding(0) var<storage, read> screen_data: ScreenData;
@group(1) @binding(1) var<storage, read_write> pixel_data: array<PixelData>;
   

@group(2) @binding(0) var<storage, read> map_data : MapData;
@group(2) @binding(1) var<storage, read> tiles : array<TileData>;

@group(3) @binding(0) var<storage, read> cam_data : CamData;

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
    if in_vertex_index == 4{
        x = 1.0;
        y = -1.0;
    }
    if in_vertex_index == 5{
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
    return vec4<f32>(pixel_data[id].val_r,pixel_data[id].val_g,pixel_data[id].val_b,1.0);
    //return vec4<f32>(clr[0].val, clr[1].val, clr[2].val, 1.0);
}

fn get_rand() -> f32{
    if clr[0].up_d >= clr[1].up_d{
        clr[0].up_d = 0;
    }else{
        clr[0].up_d += 1; 
    }
    return rand_val[clr[0].up_d];
}

@compute @workgroup_size(1) fn cs_main(
@builtin(global_invocation_id) id : vec3<u32>
){

    let angl = get_angle(id.x);

    let pos = vec2(cam_data.x, cam_data.y);

    let dir = angl + cam_data.dir;

    let fill = traverse_ray(pos, dir);

    for(var i = u32(0); i < screen_data.height; i++){
        let pid = screen_data.width * i + id.x;
        pixel_data[pid].val_r = 0.;
        pixel_data[pid].val_g = 0.;
        pixel_data[pid].val_b = 0.;
    }
    if fill.typ == 0{
        let dist = f32(screen_data.height) / fill.dist * 2.;
        let strg = min(f32(screen_data.height) / fill.dist / 15., 1.);
        let ydit = min(u32(dist), screen_data.height);
        
        let yd2 = ydit / 2;
        let mdl = screen_data.height / 2;
        
        for(var i = mdl - yd2; i < mdl + yd2; i++){
            let pid = screen_data.width * i + id.x;
            pixel_data[pid].val_r = strg;
            pixel_data[pid].val_g = strg;
            pixel_data[pid].val_b = strg;
        }
    }
}

struct MapData{
    width : u32,
    heith : u32,
}
struct TileData{
    filled : u32,
    children : array<u32,4>,
    x : u32,
    y : u32,
    w : u32,
}

struct node_fill_return{
    fill : u32,
    val : u32,
    b1 : vec2<u32>,
    b2 : vec2<u32>,
}

fn return_fill(val : u32, pos : vec2<u32>) -> node_fill_return{
    return node_fill_return(0, val, pos, vec2(pos.x + 1, pos.y + 1));
}
fn return_area(pos : vec2<u32>, w : u32) -> node_fill_return{
    return node_fill_return(1, 0, pos, vec2(pos.x + w, pos.y + w));
}
fn return_err(err : u32) -> node_fill_return{
    return node_fill_return(2, err, vec2(0,0), vec2(0,0));
}

fn is_node_filled(tar_pos : vec2<u32>) -> node_fill_return{
    if tar_pos.x > map_data.width{
        return return_err(1);
    }
    if tar_pos.y > map_data.heith{
        return return_err(1);
    }

    var cur_tile = tiles[0];
    loop {
        if cur_tile.w == 1{
            if cur_tile.filled == 0{
                return return_fill(0, vec2(cur_tile.x, cur_tile.y));
            }
            return return_fill(1, vec2(cur_tile.x, cur_tile.y));
        }
        let nw = cur_tile.w / 2;

        var id_x = 1; 
        if tar_pos.x < cur_tile.x + nw { 
            id_x = 0; 
        }

        var id_y = 1; 
        if tar_pos.y < cur_tile.y + nw { 
            id_y = 0; 
        }
        let id = id_y * 2 + id_x;

        if cur_tile.children[id] == 0{
            let nx = cur_tile.x + u32(nw) * u32(id_x);
            let ny = cur_tile.y + u32(nw) * u32(id_y);
            return return_area(vec2(nx, ny), nw); 
        }

        cur_tile = tiles[cur_tile.children[id]];
    }
    return return_err(2);
}

fn degre_to_rad(val : f32) -> f32{
    return (val / 180.) * 3.14;
}

fn get_angle(id_x : u32) -> f32{
    let r_fov = f32(screen_data.width / 2);
    let step = 60.0 / r_fov;
    let x = f32(id_x) - r_fov;
    let angle = x * step;
    return angle;
}

struct ray_res{
    typ : u32,
    dist : f32,
}

fn traverse_ray(
    start_pos : vec2<f32>,
    dir : f32,
) -> ray_res{
    var pos = vec2(start_pos.x, start_pos.y);

    let angle = degre_to_rad(dir);

    let mov = vec2(sin(angle), cos(angle));

    for(var s = 0; s < 200; s+=1) {

        if i32(pos.x) < 0 || i32(pos.y) < 0 {
            return ray_res(1, -1.);
        }

        let d = is_node_filled(vec2(u32(pos.x), u32(pos.y)));
        
        if d.fill == 0{
            let dist = distance(start_pos, pos);
            return ray_res(0, dist);
        }
        if d.fill == 1{
            pos = cross_area(
                pos, 
                mov, 
                vec2(
                    f32(d.b1.x) - 0.5, 
                    f32(d.b1.y) - 0.5
                ), 
                vec2(
                    f32(d.b2.x) + 0.5, 
                    f32(d.b2.y) + 0.5
                )
            );
        }
        if d.fill == 2{
            break;
        }
    }
    return ray_res(2, 0.);
}

fn distance(a : vec2<f32>, b : vec2<f32>) -> f32{
    return sqrt(pow(a.x - b.x, 2) + pow(a.y - b.y, 2));
}

fn cross_area(pos : vec2<f32>, mov : vec2<f32>, b1 : vec2<f32>, b2 : vec2<f32>) -> vec2<f32>{
    var t = 10000.;
    if mov.x > 0.{
        t = min(t,abs(pos.x - b2.x) / abs(mov.x));
    }
    else if mov.x < 0.{
        t = min(t,abs(pos.x - b1.x) / abs(mov.x));
    }

    if mov.y > 0.{
        t = min(t,abs(pos.y - b2.y) / abs(mov.y));
    }
    else if mov.y < 0.{
        t = min(t,abs(pos.y - b1.y) / abs(mov.y));
    }

    return vec2(pos.x + mov.x * t, pos.y + mov.y * t);
}

fn next_cross_x(pos : vec2<f32>, mov : vec2<f32>) -> vec2<f32>{
    if mov.x == 0.{
        return vec2(-10.,-10.);
    }
    var nx = 0.;
    if mov.x > 0.{
        nx = floor(pos.x) + 1.;
    }else{
        nx = ceil(pos.x) - 1.;
    }
    let t = abs(pos.x - nx) / mov.x;
    return vec2(nx, pos.y + mov.y * t);
}

fn next_cross_y(pos : vec2<f32>, mov : vec2<f32>) -> vec2<f32>{
    if mov.y == 0.{
        return vec2(-10.,-10.);
    }
    var ny = 0.;
    if mov.y > 0.{
        ny = floor(pos.y) + 1.;
    }
    else {
        ny = ceil(pos.y) - 1.;
    }
    let t = abs(pos.y - ny) / mov.y;
    return vec2(pos.x + mov.x * t, ny);
}

fn render_map(id : vec3<u32>){
    let pid = screen_data.width * id.y + id.x;
    let rval = is_node_filled(vec2(id.x,id.y));
    
    var clr_r = 0.0;
    var clr_g = 0.0;
    var clr_b = 0.0;

    if rval.fill == 0{
        if rval.val == 1{
            clr_r = 1.0;
            clr_g = 1.0;
            clr_b = 1.0;
        }else{
            clr_r = 0.5;
            clr_g = 0.5;
            clr_b = 0.5;
        }
    }
    if rval.fill == 1{
        let d = rval.b2.x - rval.b1.x;
        clr_r = 0.1 * f32(d);
        clr_g = 0.0;
        clr_b = 0.0;
    }
    pixel_data[pid].val_r = clr_r;
    pixel_data[pid].val_g = clr_g;
    pixel_data[pid].val_b = clr_b;
    if u32(cam_data.x) == id.x && u32(cam_data.y) == id.y{
        pixel_data[pid].val_r = 0.5;
        pixel_data[pid].val_g = 0.0;
        pixel_data[pid].val_b = 0.5;
    }
}
