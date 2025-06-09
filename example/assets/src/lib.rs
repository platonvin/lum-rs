//! Crate with "asset library". This is unrelated to Lum renderer, and represends asset data (compiled, so voxels & triangles) for demo

pub enum Asset {
    TankBody,
    TankHead,
    TankRfLeg,
    TankLbLeg,
    TankLfLeg,
    TankRbLeg,
    BlockDirt,
    BlockGrass,
    BlockGrassNdirt,
    BlockStoneDirt,
    BlockBush,
    BlockLeaves,
    BlockIron,
    BlockLamp,
    BlockStoneBrick,
    BlockStoneBrickCracked,
    BlockStonePack,
    BlockBark,
    BlockWood,
    BlockPlanks,
}

fn load_asset(ass: Asset) -> &'static [u8] {
    match ass {
        Asset::TankBody => todo!(),
        Asset::TankHead => todo!(),
        Asset::TankRfLeg => todo!(),
        Asset::TankLbLeg => todo!(),
        Asset::TankLfLeg => todo!(),
        Asset::TankRbLeg => todo!(),
        Asset::BlockDirt => todo!(),
        Asset::BlockGrass => todo!(),
        Asset::BlockGrassNdirt => todo!(),
        Asset::BlockStoneDirt => todo!(),
        Asset::BlockBush => todo!(),
        Asset::BlockLeaves => todo!(),
        Asset::BlockIron => todo!(),
        Asset::BlockLamp => todo!(),
        Asset::BlockStoneBrick => todo!(),
        Asset::BlockStoneBrickCracked => todo!(),
        Asset::BlockStonePack => todo!(),
        Asset::BlockBark => todo!(),
        Asset::BlockWood => todo!(),
        Asset::BlockPlanks => todo!(),
    }
}
