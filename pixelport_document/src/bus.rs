use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::mem;
use std::marker::Reflect;
use mopa;
use pon::{ToPon, PropRef};
use inverse_dependencies_counter::*;

use std::fmt::Debug;
pub trait BusValue: mopa::Any + Debug + ToPon {
    fn bus_value_equals(&self, other: &Box<BusValue>) -> bool;
    fn bus_value_clone(&self) -> Box<BusValue>;
}
mopafy!(BusValue);

impl<T: PartialEq + Reflect + 'static + Debug + ToPon + Clone> BusValue for T {
    fn bus_value_equals(&self, other: &Box<BusValue>) -> bool {
        match (**other).downcast_ref::<T>() {
            Some(v) => v == self,
            None => false
        }
    }
    fn bus_value_clone(&self) -> Box<BusValue> {
        Box::new(self.clone())
    }
}

impl Clone for Box<BusValue> {
    fn clone(&self) -> Box<BusValue> {
        self.bus_value_clone()
    }
}
impl PartialEq for Box<BusValue> {
    fn eq(&self, other: &Box<BusValue>) -> bool {
        self.bus_value_equals(other)
    }
}

pub type ValueConstructor = Fn(&Bus) -> Box<BusValue>;

struct BusEntry {
    construct: Box<ValueConstructor>,
    volatile: bool
}

#[derive(Debug, PartialEq)]
pub struct InvalidatedChange {
    pub added: Vec<PropRef>,
    pub removed: Vec<PropRef>,
}

pub struct Bus {
    entries: HashMap<PropRef, BusEntry>,
    pub invalidations_log: Vec<InvalidatedChange>,
    inv_dep_counter: InverseDependenciesCounter<PropRef>
}

#[derive(PartialEq, Debug, Clone)]
pub enum BusError {
    NoSuchEntry,
    EntryOfWrongType
}

impl Bus {
    pub fn new() -> Bus {
        Bus {
            entries: HashMap::new(),
            invalidations_log: Vec::new(),
            inv_dep_counter: InverseDependenciesCounter::new()
        }
    }
    pub fn set(&mut self, key: &PropRef, dependencies: Vec<PropRef>, volatile: bool, construct: Box<ValueConstructor>) {
        let mut change = self.inv_dep_counter.set_dependencies(key, dependencies);
        let was_volatile = {
            match self.entries.entry(key.clone()) {
                Entry::Occupied(o) => {
                    let mut e = o.into_mut();
                    e.construct = construct;
                    mem::replace(&mut e.volatile, volatile)
                },
                Entry::Vacant(v) => {
                    v.insert(BusEntry {
                        construct: construct,
                        volatile: volatile
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
        self.invalidations_log.push(InvalidatedChange { added: change.added, removed: change.removed });
    }
    pub fn get(&self, key: &PropRef) -> Result<Box<BusValue>, BusError> {
        match self.entries.get(key) {
            Some(val) => Ok((*val.construct)(self)),
            None => Err(BusError::NoSuchEntry)
        }
    }
    pub fn get_typed<T: BusValue>(&self, key: &PropRef) -> Result<T, BusError> {
        match try!(self.get(key)).downcast::<T>() {
            Ok(box v) => Ok(v),
            Err(_) => Err(BusError::EntryOfWrongType)
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
}

#[test]
fn test() {
    // let mut bus: Bus<String> = Bus::new();
    //
    // #[derive(PartialEq, Debug, Clone)]
    // struct Uniforms {
    //     bones: Vec<i32>
    // }
    // bus.set(&"uniforms".to_string(), vec!["bones".to_string()], true, Box::new(|bus| {
    //     Box::new(Uniforms { bones: bus.get_typed::<Vec<i32>>(&"bones".to_string()).expect("No bones?") })
    // }));
    //
    // bus.set(&"bones".to_string(), Vec::new(), false, Box::new(|bus| Box::new(vec![5, 3, 10, 3])));
    //
    // let uniforms = bus.get_typed::<Uniforms>(&"uniforms".to_string()).expect("No uniform?");
    // assert_eq!(uniforms, Uniforms { bones: vec![5, 3, 10, 3] });
    // //assert_eq!(bus.invalidations_log, vec![ChangedNonZero { added: Vec::new(), removed: Vec::new() }]);
}
