
use std::collections::HashMap;

use pon::*;
use document::*;

use std::any::Any;
use std::marker::Reflect;


#[macro_export]
macro_rules! pon_expand_map {
    ($map:expr, $runtime:expr, $doc:expr => { }) => (());
    ($map:expr, $runtime:expr, $doc:expr => { $name:ident : $inner:tt, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                pon_expand!(v, $runtime, $doc => $inner)
            },
            None => return Err(PonRuntimeErr::RequiredFieldMissing { field: From::from(stringify!($name)) })
        };
        pon_expand_map!($map, $runtime, $doc => { $($rest)* })
    );
    ($map:expr, $runtime:expr, $doc:expr => { $name:ident : $inner:tt optional, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                Some(pon_expand!(v, $runtime, $doc => $inner))
            },
            None => None
        };
        pon_expand_map!($map, $runtime, $doc => { $($rest)* })
    );
    ($map:expr, $runtime:expr, $doc:expr => { $name:ident : $inner:tt | $default:expr, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                pon_expand!(v, $runtime, $doc => $inner)
            },
            None => From::from($default)
        };
        pon_expand_map!($map, $runtime, $doc => { $($rest)* })
    );
}

#[macro_export]
macro_rules! pon_expand {
    ($pon:expr, $runtime:expr, $doc:expr => ) => (());
    ($pon:expr, $runtime:expr, $doc:expr => ( enum { $($id:expr => $val:expr,)+ } )) => ({
        match (try!($runtime.translate::<String>($pon, $doc))).as_str() {
            $(
            $id => $val,
            )+
            val @ _ => return Err(PonRuntimeErr::EnumValueError {
                expected_on_of: vec![$(format!("{:?}", $id),)+],
                found: format!("{:?}", val)
            })
        }
    });
    ($pon:expr, $runtime:expr, $doc:expr => { $typ:ty }) => ({
        let mut map = HashMap::new();
        for (k, v) in try!($runtime.translate::<::std::collections::HashMap<String, Pon>>($pon, $doc)).iter() {
            map.insert(k.to_string(), try!($runtime.translate::<$typ>(v, $doc)));
        }
        map
    });
    ($pon:expr, $runtime:expr, $doc:expr => { $($rest:tt)* }) => (
        pon_expand_map!(&try!($runtime.translate::<::std::collections::HashMap<String, Pon>>($pon, $doc)), $runtime, $doc => { $($rest)* })
    );
    ($pon:expr, $runtime:expr, $doc:expr => [ $typ:ty ]) => ({
        let mut arr = vec![];
        for v in &try!($runtime.translate::<Vec<Pon>>($pon, $doc)) {
            arr.push(try!($runtime.translate::<$typ>(v, $doc)));
        }
        arr
    });
    ($pon:expr, $runtime:expr, $doc:expr => ( $typ:ty )) => (
        try!($runtime.translate::<$typ>($pon, $doc))
    );
    ($pon:expr, $runtime:expr, $doc:expr => $name:ident : $t:tt) => (
        let $name = pon_expand!($pon, $runtime, $doc => $t);
    );
}

#[macro_export]
macro_rules! pon_register_functions {
    ($runtime:expr => $($func_name:ident($($args:tt)*) {$($env_ident:ident: $env:expr),*} $ret:ty => $body:block)*) => (
        $({
            fn $func_name(pon: &Pon, runtime: &PonRuntime, document: &Document) -> Result<Box<PonNativeObject>, PonRuntimeErr> {
                pon_expand!(pon, runtime, document => $($args)*);
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
    func: Box<Fn(&Pon, &PonRuntime, &Document) -> Result<Box<PonNativeObject>, PonRuntimeErr>>,
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
        where F: Fn(&Pon, &PonRuntime, &Document) -> Result<Box<PonNativeObject>, PonRuntimeErr> + 'static {
        self.functions.insert(name.to_string(), PonFn {
            func: Box::new(func),
            target_type_name: target_type_name.to_string()
        });
    }
    pub fn translate<T: Clone + Reflect + 'static>(&self, pon: &Pon, document: &Document) -> Result<T, PonRuntimeErr> {
        match try!(self.translate_raw(pon, document)).as_any().downcast_ref::<T>() {
            Some(v) => Ok(v.clone()),
            None => {
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
    pub fn translate_raw(&self, pon: &Pon, document: &Document) -> Result<Box<PonNativeObject>, PonRuntimeErr> {
        match pon {
            &Pon::PonCall(box PonCall { ref function_name, ref arg }) => {
                match self.functions.get(function_name) {
                    Some(func) => match (*func.func)(arg, self, document) {
                        Ok(val) => Ok(val),
                        Err(err) => {
                            Err(PonRuntimeErr::CallError { in_pon: pon.clone(), error: Box::new(err) })
                        }
                    },
                    None => Err(PonRuntimeErr::NoSuchFunction { function_name: function_name.to_string() })
                }
            },
            &Pon::DependencyReference(ref named_prop_ref, Some(ref prop_ref)) => {
                match document.get_property_raw(prop_ref.entity_id, &prop_ref.property_key) {
                    Ok(val) => Ok(val.clone_to_pno()),
                    Err(err) => Err(PonRuntimeErr::BadDependency { property: named_prop_ref.clone(), error: Box::new(err) })
                }
            },
            &Pon::DependencyReference(ref named_prop_ref, None) => panic!("Trying to translate on non-resolved dependency reference"),
            &Pon::Reference(ref named_prop_ref) => Ok(named_prop_ref.clone_to_pno()),
            &Pon::Selector(ref selector) => Ok(selector.clone_to_pno()),
            &Pon::Array(ref value) => Ok(value.clone_to_pno()),
            &Pon::Object(ref value) => Ok(value.clone_to_pno()),
            &Pon::Number(ref value) => Ok(value.clone_to_pno()),
            &Pon::String(ref value) => Ok(value.clone_to_pno()),
            &Pon::Boolean(ref value) => Ok(value.clone_to_pno()),
            &Pon::Nil => Err(PonRuntimeErr::Nil)
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
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
