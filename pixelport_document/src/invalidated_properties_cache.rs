
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use document::*;
use pon::*;
use std::mem;

#[derive(Debug)]
struct InvProp {
    dependencies: Vec<PropRef>,
    dependents: Vec<PropRef>,
    invalided_by_n: i32,
    changing: bool
}
impl InvProp {
    fn new() -> InvProp {
        InvProp {
            dependencies: Vec::new(),
            dependents: Vec::new(),
            invalided_by_n: 0,
            changing: false
        }
    }
}

#[derive(Debug)]
pub struct InvalidatedProperties {
    props: HashMap<PropRef, InvProp>
}

impl InvalidatedProperties {
    pub fn new() -> InvalidatedProperties {
        InvalidatedProperties {
            props: HashMap::new()
        }
    }
    pub fn set_dependencies(&mut self, prop_ref: &PropRef, dependencies: Vec<PropRef>) {
        // This pr depends on a all these dependencies and together they have all invalidated this
        // one `uninvalidate` number of times.
        let mut uninvalidate = 0;
        let old_dependencies = {
            mem::replace(&mut self.props.entry(prop_ref.clone()).or_insert(InvProp::new()).dependencies, dependencies.clone())
        };
        for d in old_dependencies {
            let p = self.props.entry(d).or_insert(InvProp::new());
            p.dependents.retain(|x| !x.eq(&prop_ref));
            uninvalidate += p.invalided_by_n;
        }
        // We're now updating to a new set of dependencies for this pr, and these will all have a
        // different amount of times they want to invalidate this thing
        let mut reinvalidate = 0;
        for d in dependencies {
            let p = self.props.entry(d).or_insert(InvProp::new());
            p.dependents.push(prop_ref.clone());
            reinvalidate += p.invalided_by_n;
        }
        // Finally we update the _depenedants_ of this one (which hasn't changed!) with the difference
        // in how many this one is invalidated by.
        if reinvalidate - uninvalidate != 0 {
            self.change_invalidated_by_n_recursively(prop_ref.clone(), reinvalidate - uninvalidate);
        }
    }
    pub fn set_changing(&mut self, prop_ref: &PropRef, changing: bool) {
        let was_changing = {
            mem::replace(&mut self.props.entry(prop_ref.clone()).or_insert(InvProp::new()).changing, changing)
        };
        if !was_changing && changing {
            self.change_invalidated_by_n_recursively(prop_ref.clone(), 1);
        } else if was_changing && !changing {
            self.change_invalidated_by_n_recursively(prop_ref.clone(), -1);
        }
    }
    pub fn close_cycle(&mut self) -> Vec<PropRef> {
        self.props.iter().filter_map(|(k, p)| {
            if p.invalided_by_n > 0 {
                Some(k.clone())
            } else {
                None
            }
        }).collect()
    }
    fn change_invalidated_by_n_recursively(&mut self, prop_ref: PropRef, diff: i32) {
        let dependents = {
            let p = self.props.entry(prop_ref).or_insert(InvProp::new());
            p.invalided_by_n += diff;
            p.dependents.clone()
        };
        for pr in dependents {
            self.change_invalidated_by_n_recursively(pr, diff);
        }
    }
}

#[derive(Debug)]
pub struct InvalidatedPropertiesCache {
    props: InvalidatedProperties,
    last_changed: HashMap<PropRef, u64>,
    cycle: u64
}
impl InvalidatedPropertiesCache {
    pub fn new() -> InvalidatedPropertiesCache {
        InvalidatedPropertiesCache {
            props: InvalidatedProperties::new(),
            last_changed: HashMap::new(),
            cycle: 2
        }
    }
    pub fn on_property_set(&mut self, prop_ref: &PropRef, dependencies: Vec<PropRef>) {
        self.props.set_dependencies(prop_ref, dependencies);
        self.last_changed.insert(prop_ref.clone(), self.cycle);
    }
    pub fn close_cycle(&mut self) -> Vec<PropRef> {
        for (prop_ref, last_changed) in self.last_changed.iter() {
            if *last_changed == self.cycle {
                self.props.set_changing(prop_ref, true);
            } else if *last_changed == self.cycle - 1 {
                self.props.set_changing(prop_ref, false);
            }
        }
        self.cycle += 1;
        self.props.close_cycle()
    }
}
