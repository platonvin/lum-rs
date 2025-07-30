//! Module with all visible in API types (the ones that do not change based on backend)

// vec4 > Vec<f32, 4>
#![allow(non_camel_case_types)]

use std::num::NonZeroUsize;

use crate::BLOCK_SIZE;
use qvek::vek;

// my glsl brain dictaited me to do this
pub type uvec4 = vek::Vec4<u32>;
pub type u16vec4 = vek::Vec4<u16>;
pub type u8vec4 = vek::Vec4<u8>;
pub type uvec3 = vek::Vec3<u32>;
pub type u16vec3 = vek::Vec3<u16>;
pub type u8vec3 = vek::Vec3<u8>;
pub type uvec2 = vek::Vec2<u32>;
pub type u16vec2 = vek::Vec2<u16>;
pub type u8vec2 = vek::Vec2<u8>;

pub type ivec4 = vek::Vec4<i32>;
pub type i16vec4 = vek::Vec4<i16>;
pub type i8vec4 = vek::Vec4<i8>;
pub type ivec3 = vek::Vec3<i32>;
pub type i16vec3 = vek::Vec3<i16>;
pub type i8vec3 = vek::Vec3<i8>;
pub type ivec2 = vek::Vec2<i32>;
pub type i16vec2 = vek::Vec2<i16>;
pub type i8vec2 = vek::Vec2<i8>;

pub type vec4 = vek::Vec4<f32>;
pub type vec3 = vek::Vec3<f32>;
pub type vec2 = vek::Vec2<f32>;

pub type dvec4 = vek::Vec4<f64>;
pub type dvec3 = vek::Vec3<f64>;
pub type dvec2 = vek::Vec2<f64>;

pub type mat4 = vek::Mat4<f32>;
pub type dmat4 = vek::Mat4<f64>;
pub type quat = vek::quaternion::Quaternion<f32>;
pub type dquat = vek::quaternion::Quaternion<f64>;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct VoxelForContour<V: PartialEq>(pub V);

#[repr(C)]
#[derive(as_u8_slice_derive::AsU8Slice, Default, Clone, Copy)]
pub struct Material {
    pub albedo: vec3,
    pub transparency: f32,
    pub emmitness: f32,
    pub roughness: f32,
}
impl std::fmt::Debug for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2})",
            self.albedo.x,
            self.albedo.y,
            self.albedo.z,
            self.transparency,
            self.emmitness,
            self.roughness,
        )
    }
}

/// API BlockId type - same across all backends. Used in API and CPU-side world representation (GPU-side does NOT use this exact type)
pub type BlockId = i16;
// Material ID and Voxel are essentially the same thing
pub type MatId = u8;
// TODO: enum with empty / non-empty using NonZeroU8
pub type Voxel = u8;

pub type MeshBlock = i16;

// taken from niche_types
// TODO: look for feature
macro_rules! define_valid_range_type {
    ($(
        $(#[$m:meta])*
        $vis:vis struct $name:ident($int:ident as $uint:ident in $low:literal..=$high:literal);
    )+) => {$(
        #[derive(Clone, Copy, Eq)]
        #[repr(transparent)]
        #[rustc_layout_scalar_valid_range_start($low)]
        #[rustc_layout_scalar_valid_range_end($high)]
        $(#[$m])*
        $vis struct $name($int);

        const _: () = {
            // With the `valid_range` attributes, it's always specified as unsigned
            assert!(<$uint>::MIN == 0);
            let ulow: $uint = $low;
            let uhigh: $uint = $high;
            assert!(ulow <= uhigh);

            assert!(size_of::<$int>() == size_of::<$uint>());
        };

        impl $name {
            #[inline]
            pub const fn new(val: $int) -> Option<Self> {
                if (val as $uint) >= ($low as $uint) && (val as $uint) <= ($high as $uint) {
                    // SAFETY: just checked the inclusive range
                    Some(unsafe { $name(val) })
                } else {
                    None
                }
            }

            /// Constructs an instance of this type from the underlying integer
            /// primitive without checking whether its zero.
            ///
            /// # Safety
            /// Immediate language UB if `val == 0`, as it violates the validity
            /// invariant of this type.
            #[inline]
            pub const unsafe fn new_unchecked(val: $int) -> Self {
                // SAFETY: Caller promised that `val` is non-zero.
                unsafe { $name(val) }
            }

            #[inline]
            pub const fn as_inner(self) -> $int {
                // SAFETY: This is a transparent wrapper, so unwrapping it is sound
                // (Not using `.0` due to MCP#807.)
                unsafe { std::mem::transmute(self) }
            }
        }

        // This is required to allow matching a constant.  We don't get it from a derive
        // because the derived `PartialEq` would do a field projection, which is banned
        // by <https://github.com/rust-lang/compiler-team/issues/807>.
        impl std::marker::StructuralPartialEq for $name {}

        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.as_inner() == other.as_inner()
            }
        }

        impl Ord for $name {
            #[inline]
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                Ord::cmp(&self.as_inner(), &other.as_inner())
            }
        }

        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(Ord::cmp(self, other))
            }
        }

        impl std::hash::Hash for $name {
            // Required method
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                std::hash::Hash::hash(&self.as_inner(), state);
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                <$int as std::fmt::Debug>::fmt(&self.as_inner(), f)
            }
        }
    )+};
}

define_valid_range_type! {
    pub struct NonZeroU8Inner(u8 as u8 in 1..=0xff);
    pub struct NonZeroU16Inner(u16 as u16 in 1..=0xff_ff);
    pub struct NonZeroU32Inner(u32 as u32 in 1..=0xffff_ffff);
    pub struct NonZeroU64Inner(u64 as u64 in 1..=0xffffffff_ffffffff);
    pub struct NonZeroU128Inner(u128 as u128 in 1..=0xffffffffffffffff_ffffffffffffffff);

    pub struct NonZeroI8Inner(i8 as u8 in 1..=0xff);
    pub struct NonZeroI16Inner(i16 as u16 in 1..=0xff_ff);
    pub struct NonZeroI32Inner(i32 as u32 in 1..=0xffff_ffff);
    pub struct NonZeroI64Inner(i64 as u64 in 1..=0xffffffff_ffffffff);
    pub struct NonZeroI128Inner(i128 as u128 in 1..=0xffffffffffffffff_ffffffffffffffff);

    pub struct NonZeroCharInner(char as u32 in 1..=0x10ffff);
}

#[cfg(target_pointer_width = "16")]
define_valid_range_type! {
    pub struct UsizeNoHighBit(usize as usize in 0..=0x7fff);
    pub struct NonZeroUsizeInner(usize as usize in 1..=0xffff);
    pub struct NonZeroIsizeInner(isize as usize in 1..=0xffff);
}
#[cfg(target_pointer_width = "32")]
define_valid_range_type! {
    pub struct UsizeNoHighBit(usize as usize in 0..=0x7fff_ffff);
    pub struct NonZeroUsizeInner(usize as usize in 1..=0xffff_ffff);
    pub struct NonZeroIsizeInner(isize as usize in 1..=0xffff_ffff);
}
#[cfg(target_pointer_width = "64")]
define_valid_range_type! {
    pub struct UsizeNoHighBit(usize as usize in 0..=0x7fff_ffff_ffff_ffff);
    pub struct NonZeroUsizeInner(usize as usize in 1..=0xffff_ffff_ffff_ffff);
    pub struct NonZeroIsizeInner(isize as usize in 1..=0xffff_ffff_ffff_ffff);
}

impl From<UsizeNoHighBit> for usize {
    fn from(value: UsizeNoHighBit) -> Self {
        value.as_inner()
    }
}

impl TryInto<UsizeNoHighBit> for usize {
    type Error = ();

    fn try_into(self) -> Result<UsizeNoHighBit, Self::Error> {
        match UsizeNoHighBit::new(self) {
            Some(value) => Ok(value),
            None => Err(()),
        }
    }
}

pub type MeshModel = UsizeNoHighBit;
pub type MeshVolumetric = UsizeNoHighBit;
pub type MeshLiquid = UsizeNoHighBit;
pub type MeshFoliage = UsizeNoHighBit;

// I am unsure about if this should be shared between backends but it is at the moment
/// CPU-side particle (grid-aligned but not grid-snapped cube with material and size, dependent on lifetime)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    pub pos: vec3,
    pub vel: vec3,
    pub life_time: f32,
    pub mat_id: MatId,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct MeshTransform {
    pub rotation: quat,
    pub translation: vec3,
}

pub type BlockVoxels = [[[Voxel; BLOCK_SIZE as usize]; BLOCK_SIZE as usize]; BLOCK_SIZE as usize];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AoLut {
    pub world_shift: vec3,
    pub weight_normalized: f32, // ((1-r^2)/total_weight)*0.7
    pub screen_shift: vec2,
    pub padding: vec2,
}
