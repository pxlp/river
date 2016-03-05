#![feature(plugin, box_patterns, slice_concat_ext, core_intrinsics, reflect_marker, vec_push_all)]
#![plugin(peg_syntax_ext)]

extern crate xml;
extern crate cgmath;
#[macro_use]
extern crate log;
#[macro_use]
extern crate mopa;

#[macro_use]
pub mod hashmap_macro;
#[macro_use]
pub mod pon;
#[macro_use]
pub mod pon_runtime;
pub mod document;
pub mod selector;
pub mod selection;
pub mod entity_match;
mod inverse_dependencies_counter;
pub mod bus;
pub mod topic;
//mod properties;

pub use pon::*;
#[macro_use]
pub use pon_runtime::*;
pub use document::*;
pub use selector::*;
pub use selection::*;
pub use entity_match::*;
pub use bus::*;
use std::hash::Hasher;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32
}
//impl_pno!(Rectangle);
impl Hash for Rectangle {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        let str = format!("{:?}", self);
        str.hash(state);
    }
}
