#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;


#[test]
fn test_remove_entity() {
    let mut doc = Document::from_string(PonTranslater::new(), r#"<Root><Entity name="tmp" x="5.0" /></Root>"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.remove_entity(ent), Ok(()));
}

#[test]
fn test_property_get() {
    let mut doc = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.get_property::<f32>(ent, "x").unwrap(), 5.0);
}

#[test]
fn test_property_set() {
    let mut doc = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    {
        doc.set_property(ent, "x", Pon::Number(9.0), false).unwrap();
    }
    assert_eq!(doc.get_property::<f32>(ent, "x").unwrap(), 9.0);
}

#[test]
fn test_property_reference_straight() {
    let mut doc = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" y="@this.x" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 5.0);
}

#[test]
fn test_property_reference_object() {
    let mut translater = PonTranslater::new();
    pon_register_functions!(translater =>
        testy({ some: (f32), }) {} f32 => { Ok(some*2.0) }
    );
    let mut doc = Document::from_string(translater, r#"<Entity name="tmp" x="5.0" y="testy { some: @this.x }" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 10.0);
}

#[test]
fn test_property_reference_transfer() {
    let mut translater = PonTranslater::new();
    translater.register_function(|arg, translater, doc| {
        let x = translater.translate::<f32>(arg, doc).unwrap();
        Ok(Box::new(x * 2.0))
    }, PonDocFunction { name: "something".to_string(), target_type_name: "f32".to_string(), arg: PonDocMatcher::Value { typ: "f32".to_string() } });
    let mut doc = Document::from_string(translater, r#"<Entity name="tmp" x="5.0" y="something @this.x" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 10.0);
}

#[test]
fn test_property_reference_array() {
    let mut translater = PonTranslater::new();
    pon_register_functions!(translater =>
        testy(some: [f32]) {} f32 => { Ok(some[0]*2.0) }
    );
    let mut doc = Document::from_string(translater, r#"<Entity name="tmp" x="5.0" y="testy [@this.x]" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 10.0);
}

#[test]
fn test_property_array_reference() {
    let mut translater = PonTranslater::new();
    pon_register_functions!(translater =>
        testy(some: [f32]) {} f32 => { Ok(some[0]*2.0) }
    );
    let mut doc = Document::from_string(translater, r#"<Entity name="tmp" x="[5.0]" y="testy @this.x" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 10.0);
}

// #[test]
// fn test_property_reference_bad_ref() {
//     let mut doc = Document::from_string(r#"<Entity name="tmp" x="5.0" y="@what.x" />"#).unwrap();
//     let ent = doc.get_entity_by_name("tmp").unwrap();
//     assert_eq!(doc.get_property::<f32>(ent, "y").err().unwrap(), DocError::NoSuchProperty("y".to_string()));
// }

#[test]
fn test_property_reference_parent() {
    let mut doc = Document::from_string(PonTranslater::new(), r#"<Entity x="5.0"><Entity name="tmp" y="@parent.x" /></Entity>"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 5.0);
}

#[test]
fn test_property_reference_update() {
    let mut doc = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" y="@this.x" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    {
        doc.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    }
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 9.0);
}


#[test]
fn test_property_reference_not_yet_created() {
    let mut doc = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" y="@this.x" />"#).unwrap();
    let ent = doc.get_entity_by_name("tmp").unwrap();
    {
        doc.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    }
    assert_eq!(doc.get_property::<f32>(ent, "y").unwrap(), 9.0);
}


#[test]
fn test_document_to_string_empty() {
    let doc = Document::new(PonTranslater::new());
    assert_eq!(doc.to_string(), "<?xml version=\"1.1\" encoding=\"UTF-8\"?>");
}
