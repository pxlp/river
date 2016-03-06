#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;


#[test]
fn test() {
    // let mut bus: Bus<String> = Bus::new();
    //
    // #[derive(PartialEq, Debug, Clone)]
    // struct Uniforms {
    //     bones: Vec<i32>
    // }
    // bus.set(&"uniforms".to_string(), vec!["bones".to_string()], true, Box::new(|bus| {
    //     Box::new(Uniforms { bones: bus.get_typed::<Vec<i32>>(&"bones".to_string()).expect("No bones?") })
    // }));
    //
    // bus.set(&"bones".to_string(), Vec::new(), false, Box::new(|bus| Box::new(vec![5, 3, 10, 3])));
    //
    // let uniforms = bus.get_typed::<Uniforms>(&"uniforms".to_string()).expect("No uniform?");
    // assert_eq!(uniforms, Uniforms { bones: vec![5, 3, 10, 3] });
    // //assert_eq!(bus.invalidations_log, vec![ChangedNonZero { added: Vec::new(), removed: Vec::new() }]);
}

#[test]
fn test_entry_of_wrong_type() {
    let mut bus: Bus = Bus::new();

    bus.set(&PropRef::new(5, "x"), Vec::new(), false, Box::new(|bus| {
        Ok(Box::new(5))
    }));
    let v = bus.get_typed::<String>(&PropRef::new(5, "x"));
    assert_eq!(v, Err(BusError::EntryOfWrongType { expected: "collections::string::String".to_string(), found: "i32".to_string(), value: "5".to_string() }));
}
