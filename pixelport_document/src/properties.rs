
use document::*;
use pon::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::mem;
use std::cell::RefCell;
use std::u64;

#[derive(Debug)]
struct InvProp {
    dependencies: Vec<PropRef>,
    dependents: Vec<PropRef>,
    counter: i32
}
impl InvProp {
    fn new() -> InvProp {
        InvProp {
            dependencies: Vec::new(),
            dependents: Vec::new(),
            counter: 0
        }
    }
}

#[derive(Debug)]
pub struct InverseDependenciesCounter {
    props: HashMap<PropRef, InvProp>
}

impl InverseDependenciesCounter {
    pub fn new() -> InverseDependenciesCounter {
        InverseDependenciesCounter {
            props: HashMap::new()
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
            uninvalidate += p.counter;
        }
        // We're now updating to a new set of dependencies for this pr, and these will all have a
        // different amount of times they want to invalidate this thing
        let mut reinvalidate = 0;
        for d in dependencies {
            let p = self.props.entry(d).or_insert(InvProp::new());
            p.dependents.push(prop_ref.clone());
            reinvalidate += p.counter;
        }
        // Finally we update the _depenedants_ of this one (which hasn't changed!) with the difference
        // in how many this one is invalidated by.
        if reinvalidate - uninvalidate != 0 {
            self.change_counter_recursively(prop_ref.clone(), reinvalidate - uninvalidate);
        }
    }
    pub fn remove_property(&mut self, prop_ref: &PropRef) {
        self.set_dependencies(prop_ref, Vec::new());
        self.props.remove(prop_ref);
    }
    pub fn get_nonzero(&self) -> Vec<PropRef> {
        self.props.iter().filter_map(|(k, p)| {
            if p.counter > 0 {
                Some(k.clone())
            } else {
                None
            }
        }).collect()
    }
    pub fn is_nonzero(&self, prop_ref: &PropRef) -> bool {
        match self.props.get(prop_ref) {
            Some(p) => p.counter > 0,
            None => false
        }
    }
    pub fn build_dependents_recursively(&mut self, prop_ref: PropRef, all_dependents: &mut Vec<PropRef>) {
        let dependents = {
            let p = self.props.entry(prop_ref).or_insert(InvProp::new());
            p.dependents.clone()
        };
        all_dependents.extend(dependents.clone());
        for pr in dependents {
            self.build_dependents_recursively(pr, all_dependents);
        }
    }
    pub fn change_counter_recursively(&mut self, prop_ref: PropRef, diff: i32) {
        let dependents = {
            let p = self.props.entry(prop_ref).or_insert(InvProp::new());
            p.counter += diff;
            p.dependents.clone()
        };
        for pr in dependents {
            self.change_counter_recursively(pr, diff);
        }
    }
}


struct CachedValue {
    value: Box<PonNativeObject>,
    valid_until_cycle: u64
}

struct Property {
    expression: Pon,
    cached_value: RefCell<Option<CachedValue>>,
    volatile: bool
}
impl Property {
    fn new() -> Property {
        Property {
            expression: Pon::Nil,
            cached_value: RefCell::new(None),
            volatile: false
        }
    }
}

pub struct Properties {
    properties: HashMap<PropRef, Property>,
    inv_dep_counter: InverseDependenciesCounter,
    cycle_changes: PropertiesCycleChanges,
    cycle: u64
}

pub struct PropertiesCycleChanges {
    pub invalidated: Vec<PropRef>,
    pub set: Vec<PropRef>,
}
impl PropertiesCycleChanges {
    pub fn new() -> PropertiesCycleChanges {
        PropertiesCycleChanges {
            invalidated: Vec::new(),
            set: Vec::new()
        }
    }
}

impl Properties {
    pub fn new() -> Properties {
        Properties {
            properties: HashMap::new(),
            inv_dep_counter: InverseDependenciesCounter::new(),
            cycle_changes: PropertiesCycleChanges::new(),
            cycle: 2
        }
    }
    pub fn close_cycle(&mut self) -> PropertiesCycleChanges {
        let mut cycle_changes = mem::replace(&mut self.cycle_changes, PropertiesCycleChanges::new());
        cycle_changes.invalidated.sort();
        cycle_changes.invalidated.dedup();
        for pr in &cycle_changes.invalidated {
            if let Some(property) = self.properties.get_mut(&pr) {
                *property.cached_value.borrow_mut() = None;
            }
        }
        let volatile_invalidated = self.inv_dep_counter.get_nonzero();
        cycle_changes.invalidated.extend(volatile_invalidated);
        self.cycle += 1;
        cycle_changes
    }
    pub fn set_property(&mut self, prop_ref: &PropRef, expression: Pon, volatile: bool) -> Result<(), DocError> {
        let mut dependencies = vec![];
        expression.build_dependencies_array(&mut dependencies);
        let was_volatile = {
            let property = self.properties.entry(prop_ref.clone()).or_insert(Property::new());
            property.expression = expression;
            let was_volatile = property.volatile;
            property.volatile = volatile;
            was_volatile
        };
        self.inv_dep_counter.set_dependencies(prop_ref, dependencies);
        if !was_volatile && volatile {
            self.inv_dep_counter.change_counter_recursively(prop_ref.clone(), 1);
        } else if was_volatile && !volatile {
            self.inv_dep_counter.change_counter_recursively(prop_ref.clone(), -1);
        }
        if !volatile {
            let mut invalidated = Vec::new();
            self.inv_dep_counter.build_dependents_recursively(prop_ref.clone(), &mut invalidated);
            self.cycle_changes.invalidated.push(prop_ref.clone());
            self.cycle_changes.invalidated.extend(invalidated);
        }
        self.cycle_changes.set.push(prop_ref.clone());
        Ok(())
    }
    pub fn get_property_raw(&self, prop_ref: &PropRef, document: &Document) -> Result<Box<PonNativeObject>, DocError> {
        match self.properties.get(prop_ref) {
            Some(property) => {
                let is_volatile = self.inv_dep_counter.is_nonzero(prop_ref);
                let cached_value = { match &mut *property.cached_value.borrow_mut() {
                    &mut Some(ref mut v) => {
                        if is_volatile && self.cycle < v.valid_until_cycle {
                            v.valid_until_cycle = self.cycle;
                        }
                        if v.valid_until_cycle >= self.cycle {
                            Some(v.value.clone_to_pno())
                        } else {
                            None
                        }
                    },
                    &mut None => None
                } };
                if let Some(cached_value) = cached_value {
                    Ok(cached_value)
                } else {
                    let new_value = match document.runtime.translate_raw(&property.expression, document) {
                        Ok(v) => v,
                        Err(err) => return Err(DocError::PonRuntimeErr {
                            err: err,
                            prop_ref: prop_ref.clone()
                        })
                    };
                    *property.cached_value.borrow_mut() = Some(CachedValue {
                        value: new_value.clone_to_pno(),
                        valid_until_cycle: if is_volatile {
                            self.cycle
                        } else {
                            u64::MAX
                        }
                    });
                    Ok(new_value)
                }
            },
            None => Err(DocError::NoSuchProperty { prop_ref: prop_ref.clone() })
        }
    }
    pub fn has_property(&self, prop_ref: &PropRef) -> bool {
        self.properties.contains_key(prop_ref)
    }
    pub fn get_property_expression(&self, prop_ref: &PropRef) -> Result<&Pon, DocError> {
        if let Some(prop) = self.properties.get(prop_ref) {
            Ok(&prop.expression)
        } else {
            Err(DocError::NoSuchProperty { prop_ref: prop_ref.clone() })
        }
    }
    pub fn get_properties_for_entity(&self, entity_id: EntityId) -> Vec<PropRef> {
        self.properties.keys().filter_map(|k| {
            if k.entity_id == entity_id {
                Some(k.clone())
            } else {
                None
            }
        }).collect()
    }
    pub fn remove_property(&mut self, prop_ref: &PropRef) {
        self.inv_dep_counter.remove_property(prop_ref);
        self.properties.remove(prop_ref);
    }
    pub fn remove_properties_for_entity(&mut self, entity_id: EntityId) {
        let props = self.get_properties_for_entity(entity_id);
        for pr in props {
            self.remove_property(&pr);
        }
    }
}
