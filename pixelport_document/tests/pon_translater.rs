#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;
use std::collections::HashMap;

fn doc_module(ret_type: &str, matcher: PonDocMatcher) -> Vec<PonDocModule> {
    vec![
        PonDocModule {
            name: "Test".to_string(),
            doc: "".to_string(),
            functions: vec![
                PonDocFunction {
                    name: "testy".to_string(),
                    module: "Test".to_string(),
                    doc: "Helps test".to_string(),
                    target_type_name: ret_type.to_string(),
                    arg: matcher
                }
            ]
        }
    ]
}

#[test]
fn test_empty() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy() f32 => { Ok(5.0) }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy ()").unwrap(), &mut bus).unwrap(), 5.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Nil));
}

#[test]
fn test_single() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy(some: (f32)) f32 => { Ok(some*5.0) }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy 5.0").unwrap(), &mut bus).unwrap(), 25.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Capture {
        var_name: "some".to_string(),
        value: Box::new(PonDocMatcher::Value {
            typ: "f32".to_string()
        })
    }));
}

#[test]
fn test_map() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy({
            some: (f32),
        }) f32 => { Ok(some*2.0) }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy { some: 3.0 }").unwrap(), &mut bus).unwrap(), 6.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Map(vec![
            PonDocMapField {
                var_name: "some".to_string(),
                optional: false,
                default: None,
                value: PonDocMatcher::Value {
                    typ: "f32".to_string()
                }
            }
        ])));
}

#[test]
fn test_map_with_default() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy({
            some: (f32),
            thing: (f32) | 4.0,
        }) f32 => { Ok(some*thing) }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy { some: 3.0 }").unwrap(), &mut bus).unwrap(), 12.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Map(vec![
            PonDocMapField {
                var_name: "some".to_string(),
                optional: false,
                default: None,
                value: PonDocMatcher::Value {
                    typ: "f32".to_string()
                }
            },
            PonDocMapField {
                var_name: "thing".to_string(),
                optional: false,
                default: Some("4.0".to_string()),
                value: PonDocMatcher::Value {
                    typ: "f32".to_string()
                }
            }
        ])));
}

#[test]
fn test_map_with_optional() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy({
            some: (f32),
            thing: (f32) optional,
        }) f32 => {
            assert_eq!(thing, None);
            Ok(some*2.0)
        }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy { some: 3.0 }").unwrap(), &mut bus).unwrap(), 6.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Map(vec![
            PonDocMapField {
                var_name: "some".to_string(),
                optional: false,
                default: None,
                value: PonDocMatcher::Value {
                    typ: "f32".to_string()
                }
            },
            PonDocMapField {
                var_name: "thing".to_string(),
                optional: true,
                default: None,
                value: PonDocMatcher::Value {
                    typ: "f32".to_string()
                }
            }
        ])));
}

#[test]
fn test_arr() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy(some: [f32]) f32 => { Ok(some[0]*some[1]*2.0) }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy [2.0, 3.0]").unwrap(), &mut bus).unwrap(), 12.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Capture {
        var_name: "some".to_string(),
        value: Box::new(PonDocMatcher::Array {
            typ: "f32".to_string()
        })
    }));
}

#[test]
fn test_map_with_array() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy({
            some: [f32],
        }) f32 => { Ok(some[0]*2.0) }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy { some: [3.0] }").unwrap(), &mut bus).unwrap(), 6.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Map(vec![
            PonDocMapField {
                var_name: "some".to_string(),
                optional: false,
                default: None,
                value: PonDocMatcher::Array {
                    typ: "f32".to_string()
                }
            }
        ])));
}

#[test]
fn test_map_as_whole() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy(some: {f32}) f32 => { Ok(some.get("a").unwrap()*some.get("b").unwrap()*2.0) }
    );
    assert_eq!(translater.translate::<f32>(&Pon::from_string("testy { a: 3, b: 4 }").unwrap(), &mut bus).unwrap(), 24.0);

    assert_eq!(translater.get_docs(), doc_module("f32", PonDocMatcher::Capture {
            var_name: "some".to_string(),
            value: Box::new(PonDocMatcher::Object {
                typ: "f32".to_string()
            })
        }));
}

#[test]
fn test_map_missing_field() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy({
            some: (f32),
        }) f32 => { Ok(some*2.0) }
    );
    assert!(translater.translate::<f32>(&Pon::from_string("testy {}").unwrap(), &mut bus).is_err());
}

#[test]
fn test_enum() {
    let mut bus = Bus::new();
    let mut translater = PonTranslater::new();
    pon_register_functions!("Test", translater =>
        "Helps test",
        testy(some: ( enum {
            "hej" => "hello".to_string(),
            "va" => "what".to_string(),
        })) String => { Ok(some) }
    );
    assert_eq!(translater.translate::<String>(&Pon::from_string("testy 'va'").unwrap(), &mut bus).unwrap(), "what".to_string());

    assert_eq!(translater.get_docs(), doc_module("String", PonDocMatcher::Capture {
            var_name: "some".to_string(),
            value: Box::new(PonDocMatcher::Enum(vec![
                PonDocEnumOption { name: "hej".to_string() },
                PonDocEnumOption { name: "va".to_string() },
            ]))
        }));
}

#[test]
fn test_nil() {
    let mut bus = Bus::new();
    let translater = PonTranslater::new();
    assert_eq!(translater.translate::<()>(&Pon::from_string("()").unwrap(), &mut bus).unwrap(), ());
}
