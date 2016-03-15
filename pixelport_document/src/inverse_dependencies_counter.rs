use std::collections::HashMap;
use std::mem;
use std::hash::Hash;

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
    pub fn set_dependencies(&mut self, key: &K, dependencies: Vec<K>, change: &mut ChangedNonZero<K>) {
        // This pr depends on a all these dependencies and together they have all invalidated this
        // one `uninvalidate` number of times.
        let mut uninvalidate = 0;
        let old_dependencies = {
            let p = self.props.entry(key.clone()).or_insert(InvProp::new());
            if p.dependencies == dependencies {
                return;
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
            self.change_counter_recursively(key.clone(), reinvalidate - uninvalidate, change);
        }
    }
    pub fn remove_property(&mut self, key: &K) -> ChangedNonZero<K> {
        let mut change = ChangedNonZero::new();
        self.set_dependencies(key, Vec::new(), &mut change);
        self.props.remove(key);
        change
    }
    pub fn is_nonzero(&self, key: &K) -> bool {
        match self.props.get(key) {
            Some(p) => p.counter > 0,
            None => false
        }
    }
    pub fn iter_nonzero<'a>(&'a self) -> Box<Iterator<Item=&'a K> + 'a> {
        Box::new(self.props.iter().filter_map(|(k, v)| {
            if v.counter > 0 {
                Some(k)
            } else {
                None
            }
        }))
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
