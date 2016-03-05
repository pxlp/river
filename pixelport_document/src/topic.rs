
use bus::*;
use pon::*;
use document::CycleChanges;
use std::marker::PhantomData;
use std::marker::Reflect;

#[derive(Debug)]
pub struct Topic {
    invalidated: Vec<PropRef>
}
impl Topic {
    pub fn new() -> Topic {
        Topic {
            invalidated: Vec::new()
        }
    }
    pub fn invalidated<F: Fn(&Bus, &PropRef) -> bool>(&mut self, bus: &Bus, cycle_changes: &CycleChanges, filter: F) -> Vec<PropRef> {
    let mut inv = self.invalidated.clone();
        for c in &cycle_changes.invalidations_log {
            for i in &c.added {
                if filter(bus, i) {
                    self.invalidated.push(i.clone());
                    inv.push(i.clone());
                }
            }
            for i in &c.removed {
                self.invalidated.retain(|x| x != i);
            }
        }
        inv.sort();
        inv.dedup();
        inv
    }
}

#[derive(Debug)]
pub struct PropertyKeyTopic {
    topic: Topic,
    keys: Vec<String>
}

impl PropertyKeyTopic {
    pub fn new(keys: Vec<&str>) -> PropertyKeyTopic {
        PropertyKeyTopic {
            topic: Topic::new(),
            keys: keys.into_iter().map(|x| x.to_string()).collect()
        }
    }
    pub fn invalidated(&mut self, bus: &Bus, cycle_changes: &CycleChanges) -> Vec<PropRef> {
        let keys = &self.keys;
        self.topic.invalidated(bus, cycle_changes, |bus, pr| {
            keys.contains(&pr.property_key)
        })
    }
}


#[derive(Debug)]
pub struct TypeTopic<T: BusValue> {
    topic: Topic,
    phantom: PhantomData<T>
}

impl<T: BusValue> TypeTopic<T> {
    pub fn new() -> TypeTopic<T> {
        TypeTopic {
            topic: Topic::new(),
            phantom: PhantomData
        }
    }
    pub fn invalidated(&mut self, bus: &Bus, cycle_changes: &CycleChanges) -> Vec<PropRef> {
        self.topic.invalidated(bus, cycle_changes, |bus, pr| {
            match bus.get(pr) {
                Ok(v) => v.is::<T>(),
                Err(_) => false
            }
        })
    }
}
