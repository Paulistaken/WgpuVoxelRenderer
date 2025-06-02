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
