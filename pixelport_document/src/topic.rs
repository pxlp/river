
use bus::*;
use pon::*;
use document::CycleChanges;

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
        for c in &cycle_changes.invalidations_log {
            for i in &c.added {
                if filter(bus, i) {
                    self.invalidated.push(i.clone());
                }
            }
        }
        let inv = self.invalidated.clone();
        for c in &cycle_changes.invalidations_log {
            for i in &c.removed {
                self.invalidated.retain(|x| x != i);
            }
        }
        inv
    }
}

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
