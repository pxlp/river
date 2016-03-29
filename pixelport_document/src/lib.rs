#![feature(plugin, box_patterns, slice_concat_ext, core_intrinsics, reflect_marker, vec_push_all)]
#![plugin(peg_syntax_ext)]

extern crate xml;
extern crate cgmath;
#[macro_use]
extern crate log;
#[macro_use]
extern crate mopa;
extern crate pad;
extern crate regex;
extern crate serde_json;

#[macro_use]
pub mod hashmap_macro;
#[macro_use]
pub mod pon;
#[macro_use]
pub mod pon_doc;
#[macro_use]
pub mod pon_translater;
pub mod document;
pub mod selector;
pub mod selection;
pub mod entity_match;
mod inverse_dependencies_counter;
pub mod bus;
pub mod topic;
pub mod channel;
pub mod document_channels;
mod doc_stream;

pub use pon::*;
#[macro_use]
pub use pon_doc::*;
#[macro_use]
pub use pon_translater::*;
pub use document::*;
pub use selector::*;
pub use selection::*;
pub use entity_match::*;
pub use bus::*;
pub use topic::*;
pub use channel::*;
pub use document_channels::*;
