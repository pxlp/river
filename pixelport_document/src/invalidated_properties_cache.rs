
use std::collections::HashMap;
use std::collections::HashSet;
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
    props: HashMap<PropRef, InvProp>,
    cached_invalidated: Vec<PropRef> // Could be calculated from props each close_cycle but faster to cache
}

impl InvalidatedProperties {
    pub fn new() -> InvalidatedProperties {
        InvalidatedProperties {
            props: HashMap::new(),
            cached_invalidated: Vec::new()
        }
    }
    pub fn set_dependencies(&mut self, prop_ref: &PropRef, dependencies: Vec<PropRef>) {
        // This pr depends on a all these dependencies and together they have all invalidated this
        // one `uninvalidate` number of times.
        let mut uninvalidate = 0;
        let old_dependencies = {
            let p = self.props.entry(prop_ref.clone()).or_insert(InvProp::new());
            if p.dependencies == dependencies {
                return;
            }
            mem::replace(&mut p.dependencies, dependencies.clone())
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
        self.cached_invalidated.clone()
    }
    fn change_invalidated_by_n_recursively(&mut self, prop_ref: PropRef, diff: i32) {
        let dependents = {
            let p = self.props.entry(prop_ref.clone()).or_insert(InvProp::new());
            let was_invalidated = p.invalided_by_n > 0;
            p.invalided_by_n += diff;
            let is_invalidated = p.invalided_by_n > 0;
            if was_invalidated && !is_invalidated {
                self.cached_invalidated.retain(|x| !x.eq(&prop_ref));
            } else if !was_invalidated && is_invalidated {
                self.cached_invalidated.push(prop_ref);
            }
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
    to_unset_changing: HashSet<PropRef>
}
impl InvalidatedPropertiesCache {
    pub fn new() -> InvalidatedPropertiesCache {
        InvalidatedPropertiesCache {
            props: InvalidatedProperties::new(),
            to_unset_changing: HashSet::new()
        }
    }
    pub fn on_property_set(&mut self, prop_ref: &PropRef, dependencies: Vec<PropRef>, volatile: bool) {
        self.props.set_dependencies(prop_ref, dependencies);
        self.props.set_changing(prop_ref, true);
        if !volatile {
            self.to_unset_changing.insert(prop_ref.clone());
        }
    }
    pub fn close_cycle(&mut self) -> Vec<PropRef> {
        let cycle = self.props.close_cycle();
        for pr in self.to_unset_changing.drain() {
            self.props.set_changing(&pr, false);
        }
        cycle
    }
}
