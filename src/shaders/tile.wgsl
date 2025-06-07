struct MapData{
    width : u32,
    heith : u32,
}
struct TileData{
    filled : u32,
    children : array<u32;4>,
    x : u32,
    y : u32,
    w : u32,
}

@group(2) @binding(0) var<storage, read> map_data : MapData;
@group(2) @binding(1) var<storage, read> tiles : array<TileData>;

fn is_node_filled(tar_pos : vec2<u32>) -> u32{
    var cur_tile = tiles[0];
    loop {
        let nw = cur_tile.w / 2;
        let id_x = if tar_pos.x < cur_tile.x + nw { 0 } else { 1 };
        let id_y = if tar_pos.y < cur_tile.y + nw { 0 } else { 1 };
        break;
    }
    return 0;
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

