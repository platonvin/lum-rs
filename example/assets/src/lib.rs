use lum::{
    load_interface::SceneData,
    types::{ivec3, BlockId},
};

include!(concat!(env!("OUT_DIR"), "/asset_enums.rs"));

pub fn get_block(id: BlockAsset) -> BlockData<'static> {
    id.load()
}

pub fn get_model(id: ModelAsset) -> ModelData<'static> {
    id.load()
}

pub fn get_scene() -> SceneData<'static> {
    const SCENE_BYTES: &'static [u8] =
        include_bytes_aligned!(16, concat!(env!("CARGO_MANIFEST_DIR"), "/assets/scene"));

    const HEADER_SIZE: usize = size_of::<ivec3>();

    const _: () = if SCENE_BYTES.len() < HEADER_SIZE {
        panic!("Scene file is smaller than its header!")
    };

    let (header_bytes, body_bytes) = SCENE_BYTES.split_at(HEADER_SIZE);

    assert!(body_bytes.len() % size_of::<BlockId>() == 0);

    unsafe {
        let size = ivec3::from(*(header_bytes.as_ptr() as *const [i32; 3]));

        let blocks = std::slice::from_raw_parts(
            body_bytes.as_ptr() as *const BlockId,
            body_bytes.len() / size_of::<BlockId>(),
        );

        assert_eq!(
            (size.x * size.y * size.z) as usize,
            blocks.len(),
            "Scene data corruption: size in header does not match the number of blocks."
        );

        SceneData { size, blocks }
    }
}

/// Loads the embedded global material palette.
/// Returns a static reference to an array of 256 materials.
pub fn get_palette() -> &'static [Material; 256] {
    // The alignment of the Material struct is 4 bytes (from Vec3<f32>).
    // Using 16-byte alignment for safety and potential SIMD performance.
    let bytes = include_bytes_aligned!(16, concat!(env!("OUT_DIR"), "/palette.bin"));
    unsafe { &*(bytes.as_ptr() as *const [Material; 256]) }
}
