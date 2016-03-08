use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::mem;
use std::marker::Reflect;
use mopa;
use pon::{PropRef};
use pon_translater::PonTranslaterErr;
use inverse_dependencies_counter::*;
use std::cell::RefCell;

use std::fmt::Debug;
pub trait BusValue: mopa::Any + Debug {
    fn bus_value_equals(&self, other: &Box<BusValue>) -> bool;
    fn bus_value_clone(&self) -> Box<BusValue>;
    fn bus_value_type_name(&self) -> &str;
}
mopafy!(BusValue);

impl<T: PartialEq + Reflect + 'static + Debug + Clone> BusValue for T {
    fn bus_value_equals(&self, other: &Box<BusValue>) -> bool {
        match (**other).downcast_ref::<T>() {
            Some(v) => v == self,
            None => false
        }
    }
    fn bus_value_clone(&self) -> Box<BusValue> {
        Box::new(self.clone())
    }
    fn bus_value_type_name(&self) -> &str {
        unsafe {
            ::std::intrinsics::type_name::<T>()
        }
    }
}

impl Clone for Box<BusValue> {
    fn clone(&self) -> Box<BusValue> {
        (*self).bus_value_clone()
    }
}
impl PartialEq for Box<BusValue> {
    fn eq(&self, other: &Box<BusValue>) -> bool {
        self.bus_value_equals(other)
    }
}

pub type ValueConstructor = Fn(&Bus) -> Result<Box<BusValue>, BusError>;

struct BusEntry {
    construct: Box<ValueConstructor>,
    volatile: bool,
    cached: RefCell<Result<Box<BusValue>, BusError>>,
    cached_until: RefCell<u64>
}

#[derive(Debug, PartialEq)]
pub struct InvalidatedChange {
    pub added: Vec<PropRef>,
    pub removed: Vec<PropRef>,
}

pub struct Bus {
    entries: HashMap<PropRef, BusEntry>,
    pub invalidations_log: Vec<InvalidatedChange>,
    inv_dep_counter: InverseDependenciesCounter<PropRef>,
    cycle: u64
}

#[derive(PartialEq, Debug, Clone)]
pub enum BusError {
    NoSuchEntry { prop_ref: PropRef },
    EntryOfWrongType { expected: String, found: String, value: String },
    PonTranslateError { err: PonTranslaterErr, prop_ref: PropRef },
    NoConstructedYet
}
impl ToString for BusError {
    fn to_string(&self) -> String {
        match self {
            &BusError::PonTranslateError { ref err, ref prop_ref } =>
                format!("Pon translate error in {}.{}: {}", prop_ref.entity_id, prop_ref.property_key, err.to_string()),
            _ => format!("{:?}", self)
        }
    }
}


impl Bus {
    pub fn new() -> Bus {
        Bus {
            entries: HashMap::new(),
            invalidations_log: Vec::new(),
            inv_dep_counter: InverseDependenciesCounter::new(),
            cycle: 1
        }
    }
    pub fn set(&mut self, key: &PropRef, dependencies: Vec<PropRef>, volatile: bool, construct: Box<ValueConstructor>) {
        let mut change = self.inv_dep_counter.set_dependencies(key, dependencies);
        let was_volatile = {
            match self.entries.entry(key.clone()) {
                Entry::Occupied(o) => {
                    let mut e = o.into_mut();
                    e.construct = construct;
                    *e.cached_until.borrow_mut() = 0;
                    mem::replace(&mut e.volatile, volatile)
                },
                Entry::Vacant(v) => {
                    v.insert(BusEntry {
                        construct: construct,
                        volatile: volatile,
                        cached: RefCell::new(Err(BusError::NoConstructedYet)),
                        cached_until: RefCell::new(0)
                    });
                    false
                }
            }
        };
        if volatile {
            if !was_volatile {
                self.inv_dep_counter.change_counter_recursively(key.clone(), 1, &mut change);
            }
        } else {
            if was_volatile {
                self.inv_dep_counter.change_counter_recursively(key.clone(), -1, &mut change);
            } else {
                self.inv_dep_counter.change_counter_recursively(key.clone(), 1, &mut change);
                self.inv_dep_counter.change_counter_recursively(key.clone(), -1, &mut change);
            }
        }
        if change.added.len() > 0 || change.removed.len() > 0 {
            self.invalidations_log.push(InvalidatedChange { added: change.added, removed: change.removed });
        }
    }
    pub fn get(&self, key: &PropRef) -> Result<Box<BusValue>, BusError> {
        match self.entries.get(key) {
            Some(val) => {
                if *val.cached_until.borrow() >= self.cycle {
                    return match &*val.cached.borrow() {
                        &Ok(ref v) => Ok((**v).bus_value_clone()),
                        &Err(ref err) => Err(err.clone())
                    }
                }
                let v = (*val.construct)(self);
                *val.cached.borrow_mut() = match &v {
                    &Ok(ref v) => Ok((**v).bus_value_clone()),
                    &Err(ref err) => Err(err.clone())
                };
                *val.cached_until.borrow_mut() = self.cycle;
                v
            },
            None => Err(BusError::NoSuchEntry { prop_ref: key.clone() })
        }
    }
    pub fn get_typed<T: BusValue>(&self, key: &PropRef) -> Result<T, BusError> {
        match try!(self.get(key)).downcast::<T>() {
            Ok(box v) => Ok(v),
            Err(v) => {
                let expected_type_name = unsafe {
                    ::std::intrinsics::type_name::<T>()
                };
                Err(BusError::EntryOfWrongType { expected: expected_type_name.to_string(), found: (*v).bus_value_type_name().to_string(), value: format!("{:?}", v) })
            }
        }
    }
    pub fn remove(&mut self, key: &PropRef) {
        self.inv_dep_counter.set_dependencies(key, Vec::new());
        self.entries.remove(key);
    }
    pub fn has(&self, key: &PropRef) -> bool {
        self.entries.contains_key(key)
    }
    pub fn iter<'a>(&'a self) -> Box<Iterator<Item=&'a PropRef> + 'a> {
        Box::new(self.entries.keys())
    }
    pub fn clear_cache(&mut self) {
        self.cycle += 1;
    }
}
