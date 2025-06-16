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
    x: f32,
    y: f32,
    z: f32,
    d: i32,
}

