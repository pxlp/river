peg_file! pon_peg("pon.rustpeg");

pub use pon::pon_peg::ParseError as PonParseError;

use selector::*;
use entity_match::*;
use document::{Document, DocError, EntityId};

use std::collections::HashMap;
use std::slice::SliceConcatExt;
use std::hash::Hasher;
use std::hash::Hash;
use std::cmp::Eq;
use cgmath::{Vector2, Vector3, Vector4, Matrix4};

pub fn selector_from_string(string: &str) -> Result<Selector, PonParseError> {
    pon_peg::selector(string)
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedPropRef {
    pub selector: Selector,
    pub property_key: String
}
impl NamedPropRef {
    pub fn new(selector: Selector, property_key: &str) -> NamedPropRef {
        NamedPropRef {
            selector: selector,
            property_key: property_key.to_string()
        }
    }
    pub fn from_string(string: &str) -> Result<NamedPropRef, PonParseError> {
        pon_peg::propref(string)
    }
    pub fn resolve(&self, document: &Document, start_entity_id: EntityId) -> Result<PropRef, DocError> {
        let entity_id = try!(self.selector.find_first(document, start_entity_id));
        Ok(PropRef { entity_id: entity_id, property_key: self.property_key.clone() })
    }
}
impl ToString for NamedPropRef {
    fn to_string(&self) -> String {
        format!("{}.{}", self.selector.to_string(), self.property_key)
    }
}
impl ToPon for NamedPropRef {
    fn to_pon(&self) -> Pon {
        Pon::PropRef(self.clone())
    }
}

#[derive(PartialEq, Debug, Clone, Hash, PartialOrd, Ord)]
pub struct PropRef {
    pub entity_id: EntityId,
    pub property_key: String
}
impl PropRef {
    pub fn new(entity_id: EntityId, property_key: &str) -> PropRef {
        PropRef {
            entity_id: entity_id,
            property_key: property_key.to_string()
        }
    }
}
impl Eq for PropRef {
    // hack, relies on PartialEq to be defined
}

#[derive(Debug, Clone, PartialEq)]
pub struct PonCall {
    pub function_name: String,
    pub arg: Pon
}
impl PonCall {
    fn stringify(&self, options: &PonStringifyOptions) -> String {
        format!("{} {}", self.function_name.to_string(), self.arg.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pon {
    Call(Box<PonCall>),
    DepPropRef(NamedPropRef, Option<PropRef>),
    PropRef(NamedPropRef),
    Selector(Selector),
    Array(Vec<Pon>),
    Object(HashMap<String, Pon>),
    Number(f32),
    String(String),
    Boolean(bool),
    Nil
}


impl Pon {
    pub fn from_string(string: &str) -> Result<Pon, PonParseError> {
        pon_peg::body(string)
    }
    pub fn call(function_name: &str, arg: Pon) -> Pon {
        Pon::Call(Box::new(PonCall { function_name: function_name.to_string(), arg: arg }))
    }
    pub fn build_dependencies_array(&self, references: &mut Vec<PropRef>) {
        match self {
            &Pon::Call(box PonCall { ref arg, .. } ) =>
                arg.build_dependencies_array(references),
            &Pon::DepPropRef(_, ref reference) => {
                references.push(match reference {
                    &Some(ref v) => v.clone(),
                    &None => panic!("trying to run build_dependencies_array on non-resolved Pon")
                });
            },
            &Pon::Object(ref hm) => {
                for (_, v) in hm {
                    v.build_dependencies_array(references);
                }
            },
            &Pon::Array(ref arr) => {
                for v in arr {
                    v.build_dependencies_array(references);
                }
            },
            _ => {}
        }
    }

    fn stringify(&self, options: &PonStringifyOptions) -> String {
        match self {
            &Pon::Call(box ref typed_pon) => typed_pon.stringify(&options),
            &Pon::DepPropRef(ref named_prop_ref, ref resolved) => format!("@{}", named_prop_ref.to_string()),
            &Pon::PropRef(ref named_prop_ref) => format!("{}", named_prop_ref.to_string()),
            &Pon::Selector(ref selector) => format!("{}", selector.to_string()),
            &Pon::Array(ref array) => {
                let a: Vec<String> = array.iter().map(|x| x.stringify(&options)).collect();
                let mut s = a.join(", ");
                if options.break_up_lines && s.len() > 180 { s = a.join(",\n"); }
                format!("[{}]", s)
            },
            &Pon::Object(ref hm) => {
                let mut a: Vec<String> = hm.iter()
                    .map(|(k, v)| format!("{}: {}", k.to_string(), v.stringify(&options))).collect();
                a.sort_by(|a, b| a.cmp(b));
                let mut s = a.join(", ");
                if options.break_up_lines && s.len() > 180 { s = a.join(",\n"); }
                format!("{{ {} }}", s)
            },
            &Pon::Number(ref v) => format!("{}", v),
            &Pon::String(ref v) => format!("'{}'", v),
            &Pon::Boolean(ref v) => format!("{}", v),
            &Pon::Nil => "()".to_string()
        }
    }
}

pub struct PonStringifyOptions {
    break_up_lines: bool
}
impl PonStringifyOptions {
    pub fn default() -> PonStringifyOptions {
        PonStringifyOptions {
            break_up_lines: false
        }
    }
}

impl ToString for Pon {
    fn to_string(&self) -> String {
        self.stringify(&PonStringifyOptions::default())
    }
}

impl Hash for Pon {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        let str = format!("{:?}", self);
        str.hash(state);
    }
}




pub trait ToPon {
    fn to_pon(&self) -> Pon;
}

impl ToPon for Pon {
    fn to_pon(&self) -> Pon {
        self.clone()
    }
}


impl ToPon for () {
    fn to_pon(&self) -> Pon {
        Pon::Nil
    }
}
impl ToPon for bool {
    fn to_pon(&self) -> Pon {
        Pon::Boolean(*self)
    }
}
impl ToPon for f32 {
    fn to_pon(&self) -> Pon {
        Pon::Number(*self)
    }
}
impl ToPon for u8 {
    fn to_pon(&self) -> Pon {
        Pon::Number(*self as f32)
    }
}
impl ToPon for u64 {
    fn to_pon(&self) -> Pon {
        Pon::Number(*self as f32)
    }
}
impl ToPon for String {
    fn to_pon(&self) -> Pon {
        Pon::String(self.clone())
    }
}
impl<'a> ToPon for &'a str {
    fn to_pon(&self) -> Pon {
        Pon::String(self.to_string())
    }
}
impl<T: ToPon> ToPon for Vec<T> {
    fn to_pon(&self) -> Pon {
        Pon::Array(self.iter().map(|v| v.to_pon()).collect())
    }
}
impl<T: ToPon> ToPon for HashMap<String, T> {
    fn to_pon(&self) -> Pon {
        Pon::Object(self.iter().map(|(k, v)| (k.clone(), v.to_pon())).collect())
    }
}

impl ToPon for Vector2<f32> {
    fn to_pon(&self) -> Pon {
        Pon::Call(Box::new(PonCall {
            function_name: "vec2".to_string(),
            arg: Pon::Object(hashmap!(
                "x" => Pon::Number(self.x),
                "y" => Pon::Number(self.y)
            ))
        }))
    }
}

#[test]
fn test_vec2_to_pon() {
    let v = Vector2::new(1.0, 2.0);
    assert_eq!(v.to_pon(), Pon::from_string("vec2 { x: 1.0, y: 2.0 }").unwrap());
}

impl ToPon for Vector3<f32> {
    fn to_pon(&self) -> Pon {
        Pon::Call(Box::new(PonCall {
            function_name: "vec3".to_string(),
            arg: Pon::Object(hashmap!(
                "x" => Pon::Number(self.x),
                "y" => Pon::Number(self.y),
                "z" => Pon::Number(self.z)
            ))
        }))
    }
}

#[test]
fn test_vec3_to_pon() {
    let v = Vector3::new(1.0, 2.0, 3.0);
    assert_eq!(v.to_pon(), Pon::from_string("vec3 { x: 1.0, y: 2.0, z: 3.0 }").unwrap());
}

impl ToPon for Vector4<f32> {
    fn to_pon(&self) -> Pon {
        Pon::Call(Box::new(PonCall {
            function_name: "vec4".to_string(),
            arg: Pon::Object(hashmap!(
                "x" => Pon::Number(self.x),
                "y" => Pon::Number(self.y),
                "z" => Pon::Number(self.z),
                "w" => Pon::Number(self.w)
            ))
        }))
    }
}

#[test]
fn test_vec4_to_pon() {
    let v = Vector4::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v.to_pon(), Pon::from_string("vec4 { x: 1.0, y: 2.0, z: 3.0, w: 4.0 }").unwrap());
}

impl ToPon for Matrix4<f32> {
    fn to_pon(&self) -> Pon {
        Pon::Call(Box::new(PonCall {
            function_name: "matrix".to_string(),
            arg: Pon::Array(vec![
                Pon::Number(self.x.x), Pon::Number(self.x.y), Pon::Number(self.x.z), Pon::Number(self.x.w),
                Pon::Number(self.y.x), Pon::Number(self.y.y), Pon::Number(self.y.z), Pon::Number(self.y.w),
                Pon::Number(self.z.x), Pon::Number(self.z.y), Pon::Number(self.z.z), Pon::Number(self.z.w),
                Pon::Number(self.w.x), Pon::Number(self.w.y), Pon::Number(self.w.z), Pon::Number(self.w.w),
            ])
        }))
    }
}
