
use bus::*;
use pon::*;
use std::marker::PhantomData;
use std::marker::Reflect;
use pon_translater::*;

#[derive(Debug)]
pub struct Topic {
    invalidated: Vec<PropRef>,
    inited: bool
}
impl Topic {
    pub fn new() -> Topic {
        Topic {
            invalidated: Vec::new(),
            inited: false
        }
    }
    pub fn invalidated<F: Fn(&PropRef) -> bool>(&mut self, bus: &Bus, invalidations_log: &Vec<InvalidatedChange>, filter: F) -> Vec<PropRef> {
        if !self.inited {
            self.invalidated = bus.iter_invalidated().filter(|pr| filter(pr)).cloned().collect();
            self.inited = true;
        }
        let mut inv = self.invalidated.clone();
        for c in invalidations_log {
            for i in &c.added {
                if filter(i) {
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
    pub fn invalidated(&mut self, bus: &Bus, invalidations_log: &Vec<InvalidatedChange>) -> Vec<PropRef> {
        let keys = &self.keys;
        self.topic.invalidated(bus, invalidations_log, |pr| {
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
    pub fn invalidated(&mut self, bus: &Bus, translater: &PonTranslater, invalidations_log: &Vec<InvalidatedChange>) -> Vec<PropRef> {
        self.topic.invalidated(bus, invalidations_log, |pr| {
            match bus.get(pr, translater) {
                Ok(v) => v.is::<T>(),
                Err(_) => false
            }
        })
    }
}
