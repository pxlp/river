
use std::collections::HashMap;

use pon::*;
use document::*;
use bus::*;

use std::marker::Reflect;

#[macro_export]
macro_rules! pon_expand_map {
    ($map:expr, $runtime:expr, $bus:expr => { }) => (());
    ($map:expr, $runtime:expr, $bus:expr => { $name:ident : $inner:tt, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                pon_expand!(v, $runtime, $bus => $inner)
            },
            None => return Err(PonRuntimeErr::RequiredFieldMissing { field: From::from(stringify!($name)) })
        };
        pon_expand_map!($map, $runtime, $bus => { $($rest)* })
    );
    ($map:expr, $runtime:expr, $bus:expr => { $name:ident : $inner:tt optional, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                Some(pon_expand!(v, $runtime, $bus => $inner))
            },
            None => None
        };
        pon_expand_map!($map, $runtime, $bus => { $($rest)* })
    );
    ($map:expr, $runtime:expr, $bus:expr => { $name:ident : $inner:tt | $default:expr, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                pon_expand!(v, $runtime, $bus => $inner)
            },
            None => From::from($default)
        };
        pon_expand_map!($map, $runtime, $bus => { $($rest)* })
    );
}

#[macro_export]
macro_rules! pon_expand {
    ($pon:expr, $runtime:expr, $bus:expr => ) => (());
    ($pon:expr, $runtime:expr, $bus:expr => ( enum { $($id:expr => $val:expr,)+ } )) => ({
        match (try!($runtime.translate::<String>($pon, $bus))).as_str() {
            $(
            $id => $val,
            )+
            val @ _ => return Err(PonRuntimeErr::EnumValueError {
                expected_on_of: vec![$(format!("{:?}", $id),)+],
                found: format!("{:?}", val)
            })
        }
    });
    ($pon:expr, $runtime:expr, $bus:expr => { $typ:ty }) => ({
        let mut map = HashMap::new();
        for (k, v) in try!($runtime.translate::<::std::collections::HashMap<String, Pon>>($pon, $bus)).iter() {
            map.insert(k.to_string(), try!($runtime.translate::<$typ>(v, $bus)));
        }
        map
    });
    ($pon:expr, $runtime:expr, $bus:expr => { $($rest:tt)* }) => (
        pon_expand_map!(&try!($runtime.translate::<::std::collections::HashMap<String, Pon>>($pon, $bus)), $runtime, $bus => { $($rest)* })
    );
    ($pon:expr, $runtime:expr, $bus:expr => [ $typ:ty ]) => ({
        let mut arr = vec![];
        for v in &try!($runtime.translate::<Vec<Pon>>($pon, $bus)) {
            arr.push(try!($runtime.translate::<$typ>(v, $bus)));
        }
        arr
    });
    ($pon:expr, $runtime:expr, $bus:expr => ( $typ:ty )) => (
        try!($runtime.translate::<$typ>($pon, $bus))
    );
    ($pon:expr, $runtime:expr, $bus:expr => $name:ident : $t:tt) => (
        let $name = pon_expand!($pon, $runtime, $bus => $t);
    );
}

#[macro_export]
macro_rules! pon_register_functions {
    ($runtime:expr => $($func_name:ident($($args:tt)*) {$($env_ident:ident: $env:expr),*} $ret:ty => $body:block)*) => (
        $({
            fn $func_name(pon: &Pon, runtime: &PonRuntime, bus: &$crate::bus::Bus<PropRef>) -> Result<Box<$crate::bus::BusValue>, PonRuntimeErr> {
                pon_expand!(pon, runtime, bus => $($args)*);
                let val: Result<$ret, PonRuntimeErr> = $body;
                match val {
                    Ok(v) => Ok(Box::new(v)),
                    Err(err) => Err(err)
                }
            }
            $runtime.register_function(stringify!($func_name), $func_name, stringify!($ret));
        })*
    );
}


struct PonFn {
    func: Box<Fn(&Pon, &PonRuntime, &Bus<PropRef>) -> Result<Box<BusValue>, PonRuntimeErr>>,
    target_type_name: String
}

pub struct PonRuntime {
    functions: HashMap<String, PonFn>
}

impl PonRuntime {
    pub fn new() -> PonRuntime {
        PonRuntime {
            functions: HashMap::new()
        }
    }
    pub fn register_function<F>(&mut self, name: &str, func: F, target_type_name: &str)
        where F: Fn(&Pon, &PonRuntime, &Bus<PropRef>) -> Result<Box<BusValue>, PonRuntimeErr> + 'static {
        self.functions.insert(name.to_string(), PonFn {
            func: Box::new(func),
            target_type_name: target_type_name.to_string()
        });
    }
    pub fn translate<T: BusValue>(&self, pon: &Pon, bus: &Bus<PropRef>) -> Result<T, PonRuntimeErr> {
        match try!(self.translate_raw(pon, bus)).downcast::<T>() {
            Ok(box v) => Ok(v),
            Err(_) => {
                let to_type_name = unsafe {
                    ::std::intrinsics::type_name::<T>()
                };
                Err(PonRuntimeErr::ValueOfUnexpectedType {
                    found_value: pon.to_string(),
                    expected_type: to_type_name.to_string()
                })
            }
        }
    }
    pub fn translate_raw(&self, pon: &Pon, bus: &Bus<PropRef>) -> Result<Box<BusValue>, PonRuntimeErr> {
        match pon {
            &Pon::PonCall(box PonCall { ref function_name, ref arg }) => {
                match self.functions.get(function_name) {
                    Some(func) => match (*func.func)(arg, self, bus) {
                        Ok(val) => Ok(val),
                        Err(err) => {
                            Err(PonRuntimeErr::CallError { in_pon: pon.clone(), error: Box::new(err) })
                        }
                    },
                    None => Err(PonRuntimeErr::NoSuchFunction { function_name: function_name.to_string() })
                }
            },
            &Pon::DependencyReference(ref named_prop_ref, Some(ref prop_ref)) => {
                match bus.get(&prop_ref) {
                    Ok(val) => Ok(val),
                    Err(err) => unimplemented!() //Err(PonRuntimeErr::BadDependency { property: named_prop_ref.clone(), error: Box::new(err) })
                }
            },
            &Pon::DependencyReference(ref named_prop_ref, None) => panic!("Trying to translate on non-resolved dependency reference"),
            &Pon::Reference(ref named_prop_ref) => Ok(Box::new(named_prop_ref.clone())),
            &Pon::Selector(ref selector) => Ok(Box::new(selector.clone())),
            &Pon::Array(ref value) => Ok(Box::new(value.clone())),
            &Pon::Object(ref value) => Ok(Box::new(value.clone())),
            &Pon::Number(ref value) => Ok(Box::new(value.clone())),
            &Pon::String(ref value) => Ok(Box::new(value.clone())),
            &Pon::Boolean(ref value) => Ok(Box::new(value.clone())),
            &Pon::Nil => Err(PonRuntimeErr::Nil)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PonRuntimeErr {
    BadDependency { property: NamedPropRef, error: Box<DocError> },
    CallError { in_pon: Pon, error: Box<PonRuntimeErr> },
    NoSuchFunction { function_name: String },
    RequiredFieldMissing { field: String },
    ValueOfUnexpectedType { expected_type: String, found_value: String },
    EnumValueError { expected_on_of: Vec<String>, found: String },
    Nil,
    Generic(String)
}
impl PonRuntimeErr {
    fn value_unexpected_type<ExpectedT>(pon_value: &Pon) -> PonRuntimeErr {
        let to_type_name = unsafe {
            ::std::intrinsics::type_name::<ExpectedT>()
        };
        PonRuntimeErr::ValueOfUnexpectedType {
            found_value: pon_value.to_string(),
            expected_type: to_type_name.to_string()
        }
    }
}

impl ToString for PonRuntimeErr {
    fn to_string(&self) -> String {
        match self {
            &PonRuntimeErr::BadDependency { ref property, ref error } => {
                format!("Bad dependency reference \"{}\", got the following error looking it up: {}", property.to_string(), error.to_string())
            }
            &PonRuntimeErr::ValueOfUnexpectedType { ref found_value, ref expected_type } => {
                format!("Expected something of type {}, found \"{}\".", expected_type, found_value)
            }
            &PonRuntimeErr::NoSuchFunction { ref function_name } => {
                format!("No such function: {}", function_name)
            }
            &PonRuntimeErr::RequiredFieldMissing { ref field } => {
                format!("Required field \"{}\" is missing", field)
            }
            &PonRuntimeErr::CallError { ref in_pon, ref error } => {
                let p = in_pon.to_string();
                let p = if p.len() < 50 {
                    p.to_string()
                } else {
                    format!("{}...", &p[0..50])
                };
                format!("function call \"{}\" failed with error: {}", p, error.to_string())
            },
            &PonRuntimeErr::EnumValueError { ref expected_on_of, ref found } => format!("Expected one of {:?}, found {}", expected_on_of, found),
            &PonRuntimeErr::Nil => "Nil value".to_string(),
            &PonRuntimeErr::Generic(ref value) => format!("{}", value)
        }
    }
}
