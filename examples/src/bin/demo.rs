#![allow(clippy::missing_safety_doc)]

use anyhow::Result;
use lum::{LumRenderer, LumSettings};

fn main() -> Result<()> {
    print!("started");
    let settings = LumSettings::default();

    let mut lum = unsafe { LumRenderer::create(&settings) }?;
    
    // LumRenderer::
    unsafe { lum.destroy() };

    print!("finished");
    Ok(())
}