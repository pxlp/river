
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use document::*;
use pon::*;


#[derive(Debug)]
pub struct InvalidatedPropertiesCache {
    dependents: HashMap<PropRef, Vec<PropRef>>,
    dependencies: HashMap<PropRef, Vec<PropRef>>,
    invalidated: HashMap<PropRef, i32>,
    last_changed: HashMap<PropRef, u64>,
    cycle: u64
}
impl InvalidatedPropertiesCache {
    pub fn new() -> InvalidatedPropertiesCache {
        InvalidatedPropertiesCache {
            dependents: HashMap::new(),
            dependencies: HashMap::new(),
            invalidated: HashMap::new(),
            last_changed: HashMap::new(),
            cycle: 2
        }
    }
    pub fn on_property_set(&mut self, prop_ref: &PropRef, mut dependencies: Vec<PropRef>) {
        dependencies.sort();
        let needs_dependencies_update = {
            if let Some(deps) = self.dependencies.get(&prop_ref) {
                deps != &dependencies
            } else {
                true
            }
        };
        let should_invalidate = {
            match self.last_changed.get(&prop_ref) {
                Some(val) => *val < self.cycle - 1,
                None => true
            }
        };
        if needs_dependencies_update {
            let uninvalidate = match self.invalidated.get(&prop_ref) {
                Some(x) => *x,
                None => 0
            };
            // remove old dependents
            if let Some(old_dependencies) = self.dependencies.remove(&prop_ref) {
                for ref pr in &old_dependencies {
                    self.dependents.get_mut(pr).unwrap().retain(|x| x.eq(&prop_ref));
                }
            }
            // add new dependents
            let mut reinvalidate = 1;
            for pr in &dependencies {
                match self.dependents.entry(pr.clone()) {
                    Entry::Occupied(o) => {
                        o.into_mut().push(prop_ref.clone());
                    },
                    Entry::Vacant(v) => {
                        v.insert(vec![prop_ref.clone()]);
                    }
                };
                reinvalidate += match self.invalidated.get(pr) {
                    Some(x) => *x,
                    None => 0
                };
            }
            self.dependencies.insert(prop_ref.clone(), dependencies);
            if reinvalidate - uninvalidate != 0 {
                self.change_invalidated_recursively(&prop_ref, reinvalidate - uninvalidate);
            }
        } else {
            if should_invalidate {
                self.change_invalidated_recursively(&prop_ref, 1);
            }
        }
        match self.last_changed.entry(prop_ref.clone()) {
            Entry::Occupied(o) => {
                *o.into_mut() = self.cycle;
            },
            Entry::Vacant(v) => {
                v.insert(self.cycle);
            }
        }
    }
    fn change_invalidated_recursively(&mut self, prop_ref: &PropRef, diff: i32) {
        match self.invalidated.entry(prop_ref.clone()) {
            Entry::Occupied(o) => {
                let x = *o.get();
                // This should really not be commented out, but for reason it won't work with
                // it on. /noren 2015-10-06
                // if x + diff < 0 {
                //     panic!("Should not be possible to reach negative number of invalidated by");
                // } else
                if x + diff == 0 {
                    o.remove();
                } else {
                    *o.into_mut() += diff;
                }
            },
            Entry::Vacant(v) => {
                if diff > 0 {
                    v.insert(diff);
                } else {
                    // This should really not be commented out, but for reason it won't work with
                    // it on. /noren 2015-10-06
                    //panic!("Should not be possible to reach negative number of invalidated by");
                }
            }
        }
        let dependents = self.dependents.get(&prop_ref).cloned();
        if let Some(dependents) = dependents {
            for pr in dependents {
                self.change_invalidated_recursively(&pr, diff);
            }
        }
    }
    pub fn close_cycle(&mut self) -> Vec<PropRef> {
        let mut to_remove = vec![];
        for (prop_ref, last_changed) in self.last_changed.iter() {
            if *last_changed != self.cycle {
                to_remove.push(prop_ref.clone());
            }
        }
        for prop_ref in to_remove {
            self.change_invalidated_recursively(&prop_ref, -1);
            self.last_changed.remove(&prop_ref);
        }
        self.cycle += 1;
        self.invalidated.iter().filter_map(|(k, v)| {
            if *v > 0 {
                Some(k.clone())
            } else {
                None
            }
        }).collect()
    }
}
