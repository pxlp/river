
use std::collections::HashMap;

use pon::*;
use bus::*;


#[macro_export]
macro_rules! pon_expand_map {
    ($map:expr, $translater:expr, $bus:expr => { }) => (());
    ($map:expr, $translater:expr, $bus:expr => { $name:ident : $inner:tt, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                pon_expand!(v, $translater, $bus => $inner)
            },
            None => return Err(PonTranslaterErr::RequiredFieldMissing { field: From::from(stringify!($name)) })
        };
        pon_expand_map!($map, $translater, $bus => { $($rest)* })
    );
    ($map:expr, $translater:expr, $bus:expr => { $name:ident : $inner:tt optional, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                Some(pon_expand!(v, $translater, $bus => $inner))
            },
            None => None
        };
        pon_expand_map!($map, $translater, $bus => { $($rest)* })
    );
    ($map:expr, $translater:expr, $bus:expr => { $name:ident : $inner:tt | $default:expr, $($rest:tt)* }) => (
        let $name = match $map.get(stringify!($name)) {
            Some(v) => {
                pon_expand!(v, $translater, $bus => $inner)
            },
            None => From::from($default)
        };
        pon_expand_map!($map, $translater, $bus => { $($rest)* })
    );
}

#[macro_export]
macro_rules! pon_expand {
    ($pon:expr, $translater:expr, $bus:expr => ) => (());
    ($pon:expr, $translater:expr, $bus:expr => ( enum { $($id:expr => $val:expr,)+ } )) => ({
        match (try!($translater.translate::<String>($pon, $bus))).as_str() {
            $(
            $id => $val,
            )+
            val @ _ => return Err(PonTranslaterErr::EnumValueError {
                expected_on_of: vec![$(format!("{:?}", $id),)+],
                found: format!("{:?}", val)
            })
        }
    });
    ($pon:expr, $translater:expr, $bus:expr => { $typ:ty }) => ({
        let mut map = HashMap::new();
        for (k, v) in try!($translater.translate::<::std::collections::HashMap<String, Pon>>($pon, $bus)).iter() {
            map.insert(k.to_string(), try!($translater.translate::<$typ>(v, $bus)));
        }
        map
    });
    ($pon:expr, $translater:expr, $bus:expr => { $($rest:tt)* }) => (
        pon_expand_map!(&try!($translater.translate::<::std::collections::HashMap<String, Pon>>($pon, $bus)), $translater, $bus => { $($rest)* })
    );
    ($pon:expr, $translater:expr, $bus:expr => [ $typ:ty ]) => ({
        let mut arr = vec![];
        for v in &try!($translater.translate::<Vec<Pon>>($pon, $bus)) {
            arr.push(try!($translater.translate::<$typ>(v, $bus)));
        }
        arr
    });
    ($pon:expr, $translater:expr, $bus:expr => ( $typ:ty )) => (
        try!($translater.translate::<$typ>($pon, $bus))
    );
    ($pon:expr, $translater:expr, $bus:expr => $name:ident : $t:tt) => (
        let $name = pon_expand!($pon, $translater, $bus => $t);
    );
}

#[macro_export]
macro_rules! pon_register_functions {
    ($translater:expr => $($func_name:ident($($args:tt)*) {$($env_ident:ident: $env:expr),*} $ret:ty => $body:block)*) => (
        $({
            fn $func_name(pon: &Pon, translater: &PonTranslater, bus: &$crate::bus::Bus) -> Result<Box<$crate::bus::BusValue>, PonTranslaterErr> {
                pon_expand!(pon, translater, bus => $($args)*);
                let val: Result<$ret, PonTranslaterErr> = $body;
                match val {
                    Ok(v) => Ok(Box::new(v)),
                    Err(err) => Err(err)
                }
            }
            $translater.register_function(stringify!($func_name), $func_name, stringify!($ret));
        })*
    );
}


struct PonFn {
    func: Box<Fn(&Pon, &PonTranslater, &Bus) -> Result<Box<BusValue>, PonTranslaterErr>>,
    target_type_name: String
}

pub struct PonTranslater {
    functions: HashMap<String, PonFn>
}

impl PonTranslater {
    pub fn new() -> PonTranslater {
        PonTranslater {
            functions: HashMap::new()
        }
    }
    pub fn register_function<F>(&mut self, name: &str, func: F, target_type_name: &str)
        where F: Fn(&Pon, &PonTranslater, &Bus) -> Result<Box<BusValue>, PonTranslaterErr> + 'static {
        self.functions.insert(name.to_string(), PonFn {
            func: Box::new(func),
            target_type_name: target_type_name.to_string()
        });
    }
    pub fn translate<T: BusValue>(&self, pon: &Pon, bus: &Bus) -> Result<T, PonTranslaterErr> {
        match try!(self.translate_raw(pon, bus)).downcast::<T>() {
            Ok(box v) => Ok(v),
            Err(_) => {
                let to_type_name = unsafe {
                    ::std::intrinsics::type_name::<T>()
                };
                Err(PonTranslaterErr::ValueOfUnexpectedType {
                    found_value: pon.to_string(),
                    expected_type: to_type_name.to_string()
                })
            }
        }
    }
    pub fn translate_raw(&self, pon: &Pon, bus: &Bus) -> Result<Box<BusValue>, PonTranslaterErr> {
        match pon {
            &Pon::PonCall(box PonCall { ref function_name, ref arg }) => {
                match self.functions.get(function_name) {
                    Some(func) => match (*func.func)(arg, self, bus) {
                        Ok(val) => Ok(val),
                        Err(err) => {
                            Err(PonTranslaterErr::CallError { in_pon: pon.clone(), error: Box::new(err) })
                        }
                    },
                    None => Err(PonTranslaterErr::NoSuchFunction { function_name: function_name.to_string() })
                }
            },
            &Pon::DependencyReference(ref named_prop_ref, Some(ref prop_ref)) => {
                match bus.get(&prop_ref, self) {
                    Ok(val) => Ok(val),
                    Err(err @ BusError::NoSuchEntry { .. }) => Err(PonTranslaterErr::BadDependency { property: named_prop_ref.clone(), error: Box::new(err) }),
                    Err(err) => Err(PonTranslaterErr::BusError(Box::new(err)))
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
            &Pon::Nil => Ok(Box::new(()))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PonTranslaterErr {
    BadDependency { property: NamedPropRef, error: Box<BusError> },
    BusError(Box<BusError>),
    CallError { in_pon: Pon, error: Box<PonTranslaterErr> },
    NoSuchFunction { function_name: String },
    RequiredFieldMissing { field: String },
    ValueOfUnexpectedType { expected_type: String, found_value: String },
    EnumValueError { expected_on_of: Vec<String>, found: String },
    Generic(String)
}
impl PonTranslaterErr {
    fn value_unexpected_type<ExpectedT>(pon_value: &Pon) -> PonTranslaterErr {
        let to_type_name = unsafe {
            ::std::intrinsics::type_name::<ExpectedT>()
        };
        PonTranslaterErr::ValueOfUnexpectedType {
            found_value: pon_value.to_string(),
            expected_type: to_type_name.to_string()
        }
    }
}

impl ToString for PonTranslaterErr {
    fn to_string(&self) -> String {
        match self {
            &PonTranslaterErr::BadDependency { ref property, ref error } => {
                format!("Bad dependency reference \"{}\", got the following error looking it up: {}", property.to_string(), error.to_string())
            }
            &PonTranslaterErr::BusError(ref err) => {
                format!("Buss error \"{}\"", err.to_string())
            }
            &PonTranslaterErr::ValueOfUnexpectedType { ref found_value, ref expected_type } => {
                format!("Expected something of type {}, found \"{}\".", expected_type, found_value)
            }
            &PonTranslaterErr::NoSuchFunction { ref function_name } => {
                format!("No such function: {}", function_name)
            }
            &PonTranslaterErr::RequiredFieldMissing { ref field } => {
                format!("Required field \"{}\" is missing", field)
            }
            &PonTranslaterErr::CallError { ref in_pon, ref error } => {
                let p = in_pon.to_string();
                let p = if p.len() < 50 {
                    p.to_string()
                } else {
                    format!("{}...", &p[0..50])
                };
                format!("function call \"{}\" failed with error: {}", p, error.to_string())
            },
            &PonTranslaterErr::EnumValueError { ref expected_on_of, ref found } => format!("Expected one of {:?}, found {}", expected_on_of, found),
            &PonTranslaterErr::Generic(ref value) => format!("{}", value)
        }
    }
}
