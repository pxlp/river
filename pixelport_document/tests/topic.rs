#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;
use std::mem;

#[test]
fn test_type_topic() {
    let mut bus: Bus = Bus::new();

    bus.set(&PropRef::new(5, "x"), Vec::new(), false, Box::new(|bus| Ok(Box::new(5)) ));
    let mut topic: TypeTopic<i32> = TypeTopic::new();
    let cc = CycleChanges { invalidations_log: mem::replace(&mut bus.invalidations_log, Vec::new()), entities_added: Vec::new(), entities_removed: Vec::new() };
    let inv = topic.invalidated(&bus, &cc);
    assert_eq!(inv, vec![PropRef::new(5, "x")]);
}

#[test]
fn test_type_topic_volatile() {
    let mut bus: Bus = Bus::new();

    bus.set(&PropRef::new(5, "x"), Vec::new(), true, Box::new(|bus| Ok(Box::new(5)) ));
    let mut topic: TypeTopic<i32> = TypeTopic::new();
    let cc = CycleChanges { invalidations_log: mem::replace(&mut bus.invalidations_log, Vec::new()), entities_added: Vec::new(), entities_removed: Vec::new() };
    let inv = topic.invalidated(&bus, &cc);
    assert_eq!(inv, vec![PropRef::new(5, "x")]);

    let cc = CycleChanges { invalidations_log: mem::replace(&mut bus.invalidations_log, Vec::new()), entities_added: Vec::new(), entities_removed: Vec::new() };
    let inv = topic.invalidated(&bus, &cc);
    assert_eq!(inv, vec![PropRef::new(5, "x")]);
}
