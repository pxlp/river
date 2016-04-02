#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;

#[test]
fn test_entry_of_wrong_type() {
    let mut bus: Bus = Bus::new();

    bus.set_constructor(&PropRef::new(5, "x"), Vec::new(), false, Box::new(|_, _| {
        Ok(Box::new(5))
    }));
    let v = bus.get_typed::<String>(&PropRef::new(5, "x"), &PonTranslater::new());
    assert_eq!(v, Err(BusError::EntryOfWrongType { expected: "std::string::String".to_string(), found: "i32".to_string(), value: "5".to_string() }));
}

#[test]
fn test_volatile_dependency_change() {
    let mut bus: Bus = Bus::new();

    bus.set_value(&PropRef::new(5, "rotation_z"), true, Box::new(5));

    bus.set_constructor(&PropRef::new(5, "transform"), vec![PropRef::new(5, "rotation_z")], false, Box::new(|_, _| {
        Ok(Box::new(5))
    }));

    bus.set_value(&PropRef::new(5, "transform"), true, Box::new(5));

    assert_eq!(bus.invalidations_log, vec![
        InvalidatedChange { added: vec![PropRef::new(5, "rotation_z")], removed: vec![] },
        InvalidatedChange { added: vec![PropRef::new(5, "transform")], removed: vec![] },
    ])
}
