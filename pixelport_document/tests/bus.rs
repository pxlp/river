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
    assert_eq!(v, Err(BusError::EntryOfWrongType { expected: "collections::string::String".to_string(), found: "i32".to_string(), value: "5".to_string() }));
}
