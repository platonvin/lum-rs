// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::missing_safety_doc)]

use std::{cell::RefCell, process::exit};
use std::collections::HashSet;
use std::env;
use std::ffi::CStr;
use std::mem::{self, size_of, size_of_val};

use std::os::raw::c_void;
use std::ptr::copy_nonoverlapping as memcpy;

use anyhow::{anyhow, Result};
use cgmath::{vec2, vec4};
use lum::{LumRenderer, LumSettings};
// use log::*;
// use thiserror::Error;
use vulkanalia_vma::{self as vma};
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::window as vk_window;
use vulkanalia::Version;
use vulkanalia_vma::{Alloc, AllocationCreateFlags, AllocationOptions, Allocator};
use vulkanalia_vma::AllocatorOptions;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use vk::{KhrSurfaceExtension, KhrSwapchainExtension};

use lumal::*;
// use lumal::;

fn main() -> Result<()> {
    print!("started");
    let settings = LumSettings::default();

    let mut lum = unsafe { LumRenderer::create(settings) }?;
    
    // LumRenderer::
    unsafe { lum.destroy() };

    print!("finished");
    Ok(())
}