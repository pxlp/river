use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::mem;
use std::hash::Hash;
use std::marker::Reflect;
use mopa;
use pon::ToPon;

#[derive(Debug)]
struct InvProp<K> {
    dependencies: Vec<K>,
    dependents: Vec<K>,
    counter: i32
}
impl<K> InvProp<K> {
    fn new() -> InvProp<K> {
        InvProp {
            dependencies: Vec::new(),
            dependents: Vec::new(),
            counter: 0
        }
    }
}

#[derive(Debug)]
pub struct InverseDependenciesCounter<K: Eq + Hash> {
    props: HashMap<K, InvProp<K>>
}

#[derive(Debug, PartialEq)]
pub struct ChangedNonZero<K> {
    pub added: Vec<K>,
    pub removed: Vec<K>,
}
impl<K> ChangedNonZero<K> {
    pub fn new() -> ChangedNonZero<K> {
        ChangedNonZero {
            added: Vec::new(),
            removed: Vec::new(),
        }
    }
}

impl<K: Eq + Hash + Clone> InverseDependenciesCounter<K> {
    pub fn new() -> InverseDependenciesCounter<K> {
        InverseDependenciesCounter {
            props: HashMap::new()
        }
    }
    pub fn set_dependencies(&mut self, key: &K, dependencies: Vec<K>) -> ChangedNonZero<K> {
        let mut change = ChangedNonZero::new();
        // This pr depends on a all these dependencies and together they have all invalidated this
        // one `uninvalidate` number of times.
        let mut uninvalidate = 0;
        let old_dependencies = {
            let p = self.props.entry(key.clone()).or_insert(InvProp::new());
            if p.dependencies == dependencies {
                return change;
            }
            mem::replace(&mut p.dependencies, dependencies.clone())
        };
        for d in old_dependencies {
            let p = self.props.entry(d).or_insert(InvProp::new());
            p.dependents.retain(|x| !x.eq(&key));
            uninvalidate += p.counter;
        }
        // We're now updating to a new set of dependencies for this pr, and these will all have a
        // different amount of times they want to invalidate this thing
        let mut reinvalidate = 0;
        for d in dependencies {
            let p = self.props.entry(d).or_insert(InvProp::new());
            p.dependents.push(key.clone());
            reinvalidate += p.counter;
        }
        // Finally we update the _depenedants_ of this one (which hasn't changed!) with the difference
        // in how many this one is invalidated by.
        if reinvalidate - uninvalidate != 0 {
            self.change_counter_recursively(key.clone(), reinvalidate - uninvalidate, &mut change);
        }
        change
    }
    pub fn remove_property(&mut self, key: &K) -> ChangedNonZero<K> {
        let change = self.set_dependencies(key, Vec::new());
        self.props.remove(key);
        change
    }
    pub fn is_nonzero(&self, key: &K) -> bool {
        match self.props.get(key) {
            Some(p) => p.counter > 0,
            None => false
        }
    }
    pub fn change_counter_recursively(&mut self, key: K, diff: i32, change: &mut ChangedNonZero<K>) {
        let dependents = {
            let p = self.props.entry(key.clone()).or_insert(InvProp::new());
            let was_nonzero = p.counter > 0;
            p.counter += diff;
            let is_nonzero = p.counter > 0;
            if !was_nonzero && is_nonzero {
                change.added.push(key);
            } else if was_nonzero && !is_nonzero {
                change.removed.push(key);
            }
            p.dependents.clone()
        };
        for pr in dependents {
            self.change_counter_recursively(pr, diff, change);
        }
    }
}

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

pub type ValueConstructor<K> = Fn(&Bus<K>) -> Box<BusValue>;

struct BusEntry<K: Eq + Hash + Clone> {
    construct: Box<ValueConstructor<K>>,
    volatile: bool
}

pub struct Bus<K: Eq + Hash + Clone> {
    entries: HashMap<K, BusEntry<K>>,
    pub invalidations_log: Vec<ChangedNonZero<K>>,
    inv_dep_counter: InverseDependenciesCounter<K>
}

#[derive(PartialEq, Debug, Clone)]
pub enum BusError {
    NoSuchEntry,
    EntryOfWrongType
}

impl<K: Eq + Hash + Clone> Bus<K> {
    pub fn new() -> Bus<K> {
        Bus {
            entries: HashMap::new(),
            invalidations_log: Vec::new(),
            inv_dep_counter: InverseDependenciesCounter::new()
        }
    }
    pub fn set(&mut self, key: &K, dependencies: Vec<K>, volatile: bool, construct: Box<ValueConstructor<K>>) {
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
        self.invalidations_log.push(change);
    }
    pub fn get(&self, key: &K) -> Result<Box<BusValue>, BusError> {
        match self.entries.get(key) {
            Some(val) => Ok((*val.construct)(self)),
            None => Err(BusError::NoSuchEntry)
        }
    }
    pub fn get_typed<T: BusValue>(&self, key: &K) -> Result<T, BusError> {
        match try!(self.get(key)).downcast::<T>() {
            Ok(box v) => Ok(v),
            Err(_) => Err(BusError::EntryOfWrongType)
        }
    }
    pub fn remove(&mut self, key: &K) {
        self.inv_dep_counter.set_dependencies(key, Vec::new());
        self.entries.remove(key);
    }
    pub fn has(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }
    pub fn iter<'a>(&'a self) -> Box<Iterator<Item=&'a K> + 'a> {
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



pub struct Topic<K: Eq + Hash + Clone> {
    invalidated: Vec<K>,
    filter: Box<Fn(&Bus<K>, &K) -> bool>
}
impl<K: Eq + Hash + Clone> Topic<K> {
    pub fn new(filter: Box<Fn(&Bus<K>, &K) -> bool>) -> Topic<K> {
        Topic {
            invalidated: Vec::new(),
            filter: filter
        }
    }
    pub fn consume_log(&mut self, bus: &Bus<K>) -> Vec<K> {
        for c in &bus.invalidations_log {
            for i in &c.added {
                if (*self.filter)(bus, i) {
                    self.invalidated.push(i.clone());
                }
            }
        }
        let inv = self.invalidated.clone();
        for c in &bus.invalidations_log {
            for i in &c.removed {
                self.invalidated.retain(|x| x != i);
            }
        }
        inv
    }
}

#[test]
fn test_topic() {
    // let mut bus: Bus<String> = Bus::new();
    //
    // #[derive(PartialEq, Debug, Clone)]
    // struct PickerDescription {
    //     x: i32
    // }
    // #[derive(PartialEq, Debug, Clone)]
    // struct Picker {
    //     desc: PickerDescription
    // }
    // #[derive(PartialEq, Debug, Clone)]
    // struct Pickers {
    //     pickers: HashMap<String, Picker>
    // }
    // bus.set(&"hello".to_string(), Vec::new(), false, Box::new(|bus| Box::new(PickerDescription { x: 50 })));
    // let mut topic: Topic<String> = Topic::new(Box::new(|bus, key| bus.get_typed::<PickerDescription>(key).is_ok()));
    // let mut pickers = Pickers { pickers: HashMap::new() };
    // for key in topic.consume_log(&bus) {
    //     match bus.get_typed::<PickerDescription>(&key) {
    //         Ok(desc) => {
    //             let mut picker = pickers.pickers.entry(key.to_string()).or_insert(Picker { desc: PickerDescription { x: 0 } });
    //             picker.desc = desc;
    //         },
    //         Err(_) => {
    //             pickers.pickers.remove(&key);
    //         }
    //     }
    // }
    //
    // assert_eq!(pickers, Pickers { pickers: vec![("hello".to_string(), Picker { desc: PickerDescription { x: 50 } })].into_iter().collect() });
}

// --
//
// pub trait ServicesMaintainer<Desc> {
//     fn update_service(&mut self, key: &str, desc: Desc);
//     fn remove_service(&mut self, key: &str);
// }
//
// pub struct ServiceUpdater {
//     invalidated_services: Vec<String>
// }
// impl ServiceUpdater {
//     pub fn new() -> ServiceUpdater {
//         ServiceUpdater {
//             invalidated_services: Vec::new()
//         }
//     }
//     pub fn consume_log<Desc: Reflect + 'static, T: ServicesMaintainer<Desc>>(&mut self, bus: &Bus<String>, subsystem: &mut T) {
//         for c in &bus.invalidations_log {
//             for i in &c.added {
//                 if let Ok(desc) = bus.get_typed::<Desc>(i) {
//                     self.invalidated_services.push(i.to_string());
//                 }
//             }
//         }
//         for i in &self.invalidated_services {
//             if let Ok(desc) = bus.get_typed::<Desc>(i) {
//                 subsystem.update_service(&*i, desc);
//             } else {
//                 subsystem.remove_service(&*i);
//             }
//         }
//         for c in &bus.invalidations_log {
//             for i in &c.removed {
//                 self.invalidated_services.retain(|x| x != i);
//             }
//         }
//     }
// }
//
// #[test]
// fn test_service() {
//     let mut bus: Bus<String> = Bus::new();
//
//     #[derive(PartialEq, Debug, Clone)]
//     struct PickerDescription {
//         x: i32
//     }
//     #[derive(PartialEq, Debug, Clone)]
//     struct Picker {
//         desc: PickerDescription
//     }
//     #[derive(PartialEq, Debug, Clone)]
//     struct Pickers {
//         pickers: HashMap<String, Picker>
//     }
//     impl ServicesMaintainer<PickerDescription> for Pickers {
//         fn update_service(&mut self, key: &str, desc: PickerDescription) {
//             let mut picker = self.pickers.entry(key.to_string()).or_insert(Picker { desc: PickerDescription { x: 0 } });
//             picker.desc = desc;
//         }
//         fn remove_service(&mut self, key: &str) {
//             self.pickers.remove(key);
//         }
//     }
//     bus.set(&"hello".to_string(), Vec::new(), Box::new(|bus| Box::new(PickerDescription { x: 50 })), false);
//     let mut su = ServiceUpdater::new();
//     let mut pickers = Pickers { pickers: HashMap::new() };
//     su.consume_log(&bus, &mut pickers);
//
//     assert_eq!(pickers, Pickers { pickers: vec![("hello".to_string(), Picker { desc: PickerDescription { x: 50 } })].into_iter().collect() });
// }
