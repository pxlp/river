use pixelport_document::*;

use std::hash::Hasher;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq)]
pub struct OrderedF32(f32);

impl OrderedF32 {
    pub fn new(val: f32) -> OrderedF32 {
        OrderedF32(val)
    }
    pub fn to_f32(&self) -> f32 {
        self.0
    }
}

impl Eq for OrderedF32 {
}

impl Hash for OrderedF32 {
    fn hash<H>(&self, state: &mut H) where H: ::std::hash::Hasher {
        format!("{}", self.0).hash(state);
    }
}

impl ToPon for OrderedF32 {
    fn to_pon(&self) -> Pon {
        self.0.to_pon()
    }
}
