use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::mem;
use std::marker::Reflect;
use mopa;
use inverse_dependencies_counter::*;
use std::cell::RefCell;
use pon::*;
use pon_translater::*;

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

pub type ValueConstructor = Fn(&Bus, &PonTranslater) -> Result<Box<BusValue>, BusError>;

pub enum BusEntryValue {
    Constructor {
        constructor: Box<ValueConstructor>,
        cached: RefCell<Option<Box<BusValue>>>,
        cached_until: RefCell<u64>
    },
    Value(Box<BusValue>),

    // Have the Pon in the bus muddles the bus a bit. We need it though to be able to do inteligent
    // things with the entries, such as when setting the same value twice it shouldn't do anything.
    Pon {
        expression: Pon,
        cached: RefCell<Option<Box<BusValue>>>,
        cached_until: RefCell<u64>
    }
}
impl BusEntryValue {
    fn is_same(&self, other: &BusEntryValue) -> bool {
        match other {
            &BusEntryValue::Constructor { .. } => false,
            &BusEntryValue::Value(ref v1) => match self {
                &BusEntryValue::Value(ref v2) => (**v1).bus_value_equals(&*v2),
                _ => false
            },
            &BusEntryValue::Pon { expression: ref e1, .. } => match self {
                &BusEntryValue::Pon { expression: ref e2, .. } => e1 == e2,
                _ => false
            },
        }
    }
}

struct BusEntry {
    value: BusEntryValue,
    volatile: bool
}

#[derive(Debug, PartialEq)]
pub struct InvalidatedChange {
    pub added: Vec<PropRef>,
    pub removed: Vec<PropRef>,
}

#[derive(Debug)]
pub struct BusStats {
    pub n_constructs: i32,
    pub n_gets: i32,
    pub n_cache_hits: i32,
    pub n_volatile_sets: i32,
    pub n_involatile_sets: i32,
    pub n_adds: i32,
    pub n_removes: i32,
    pub n_set_value: i32,
    pub n_set_constructor: i32,
    pub n_set_pon: i32,
    pub n_skip_set: i32,
}
impl BusStats {
    pub fn new() -> BusStats {
        BusStats {
            n_constructs: 0,
            n_gets: 0,
            n_cache_hits: 0,
            n_volatile_sets: 0,
            n_involatile_sets: 0,
            n_adds: 0,
            n_removes: 0,
            n_set_value: 0,
            n_set_constructor: 0,
            n_set_pon: 0,
            n_skip_set: 0,
        }
    }
}

pub struct Bus {
    entries: HashMap<PropRef, BusEntry>,
    pub invalidations_log: Vec<InvalidatedChange>,
    inv_dep_counter: InverseDependenciesCounter<PropRef>,
    cycle: u64,
    pub stats: RefCell<BusStats>
}

#[derive(PartialEq, Debug, Clone)]
pub enum BusError {
    NoSuchEntry { prop_ref: PropRef },
    EntryOfWrongType { expected: String, found: String, value: String },
    PonTranslateError { err: PonTranslaterErr }
}
impl From<PonTranslaterErr> for BusError {
    fn from(err: PonTranslaterErr) -> BusError {
        BusError::PonTranslateError { err: err }
    }
}
impl ToString for BusError {
    fn to_string(&self) -> String {
        match self {
            &BusError::PonTranslateError { ref err } =>
                format!("Pon translate error: {}", err.to_string()),
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
            cycle: 1,
            stats: RefCell::new(BusStats::new())
        }
    }
    pub fn set_value(&mut self, key: &PropRef, volatile: bool, value: Box<BusValue>) {
        self.stats.borrow_mut().n_set_value += 1;
        self.set(key, Vec::new(), volatile, BusEntryValue::Value(value));
    }
    pub fn set_constructor(&mut self, key: &PropRef, dependencies: Vec<PropRef>, volatile: bool, construct: Box<ValueConstructor>) {
        self.stats.borrow_mut().n_set_constructor += 1;
        self.set(key, dependencies, volatile, BusEntryValue::Constructor {
            constructor: construct,
            cached: RefCell::new(None),
            cached_until: RefCell::new(0),
        });
    }
    pub fn set_pon(&mut self, key: &PropRef, volatile: bool, expression: Pon) {
        let mut dependencies = vec![];
        expression.build_dependencies_array(&mut dependencies);
        self.stats.borrow_mut().n_set_pon += 1;
        self.set(key, dependencies, volatile, BusEntryValue::Pon {
            expression: expression,
            cached: RefCell::new(None),
            cached_until: RefCell::new(0),
        });
    }
    fn set(&mut self, key: &PropRef, dependencies: Vec<PropRef>, volatile: bool, value: BusEntryValue) {
        let mut change = self.inv_dep_counter.set_dependencies(key, dependencies);
        let was_volatile = {
            match self.entries.entry(key.clone()) {
                Entry::Occupied(o) => {
                    let mut e = o.into_mut();
                    // Early exit, means we can save the whole inv added removed thing for non-volatile sets
                    if !volatile && e.value.is_same(&value) {
                        self.stats.borrow_mut().n_skip_set += 1;
                        return;
                    }
                    e.value = value;
                    mem::replace(&mut e.volatile, volatile)
                },
                Entry::Vacant(v) => {
                    v.insert(BusEntry {
                        value: value,
                        volatile: volatile
                    });
                    false
                }
            }
        };
        if volatile {
            self.stats.borrow_mut().n_volatile_sets += 1;
            if !was_volatile {
                self.inv_dep_counter.change_counter_recursively(key.clone(), 1, &mut change);
            }
        } else {
            self.stats.borrow_mut().n_involatile_sets += 1;
            if was_volatile {
                self.inv_dep_counter.change_counter_recursively(key.clone(), -1, &mut change);
            } else {
                self.inv_dep_counter.change_counter_recursively(key.clone(), 1, &mut change);
                self.inv_dep_counter.change_counter_recursively(key.clone(), -1, &mut change);
            }
        }
        if change.added.len() > 0 || change.removed.len() > 0 {
            self.stats.borrow_mut().n_adds += change.added.len() as i32;
            self.stats.borrow_mut().n_removes += change.removed.len() as i32;
            self.invalidations_log.push(InvalidatedChange { added: change.added, removed: change.removed });
        }
    }
    pub fn get(&self, key: &PropRef, pon_translater: &PonTranslater) -> Result<Box<BusValue>, BusError> {
        self.stats.borrow_mut().n_gets += 1;
        match self.entries.get(key) {
            Some(entry) => {
                match &entry.value {
                    &BusEntryValue::Constructor { ref cached, ref cached_until, ref constructor,  } => {
                        if *cached_until.borrow() >= self.cycle {
                            self.stats.borrow_mut().n_cache_hits += 1;
                            if let &Some(ref v) = &*cached.borrow() {
                                return Ok((**v).bus_value_clone());
                            }
                        }
                        self.stats.borrow_mut().n_constructs += 1;
                        let v = try!((*constructor)(self, pon_translater));
                        *cached.borrow_mut() = Some((*v).bus_value_clone());
                        *cached_until.borrow_mut() = self.cycle;
                        Ok(v)
                    },
                    &BusEntryValue::Value(ref value) => Ok((**value).bus_value_clone()),
                    &BusEntryValue::Pon { ref cached, ref cached_until, ref expression,  } => {
                        if *cached_until.borrow() >= self.cycle {
                            self.stats.borrow_mut().n_cache_hits += 1;
                            if let &Some(ref v) = &*cached.borrow() {
                                return Ok((**v).bus_value_clone());
                            }
                        }
                        self.stats.borrow_mut().n_constructs += 1;
                        let v = try!(pon_translater.translate_raw(expression, self));
                        *cached.borrow_mut() = Some((*v).bus_value_clone());
                        *cached_until.borrow_mut() = self.cycle;
                        Ok(v)
                    },
                }
            },
            None => Err(BusError::NoSuchEntry { prop_ref: key.clone() })
        }
    }
    pub fn get_typed<T: BusValue>(&self, key: &PropRef, pon_translater: &PonTranslater) -> Result<T, BusError> {
        match try!(self.get(key, pon_translater)).downcast::<T>() {
            Ok(box v) => Ok(v),
            Err(v) => {
                let expected_type_name = unsafe {
                    ::std::intrinsics::type_name::<T>()
                };
                Err(BusError::EntryOfWrongType { expected: expected_type_name.to_string(), found: (*v).bus_value_type_name().to_string(), value: format!("{:?}", v) })
            }
        }
    }
    pub fn get_entry(&self, key: &PropRef) -> Option<&BusEntryValue> {
        match self.entries.get(key) {
            Some(v) => Some(&v.value),
            None => None
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
        self.stats = RefCell::new(BusStats::new());
    }
}
