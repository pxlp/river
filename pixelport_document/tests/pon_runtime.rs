#![feature(convert)]

#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;
use std::collections::HashMap;

#[test]
fn test_empty() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy() {} f32 => { Ok(5.0) }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy ()").unwrap(), &mut bus).unwrap(), 5.0);
}

#[test]
fn test_single() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy(some: (f32)) {} f32 => { Ok(some*5.0) }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy 5.0").unwrap(), &mut bus).unwrap(), 25.0);
}

#[test]
fn test_map() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy({
            some: (f32),
        }) {} f32 => { Ok(some*2.0) }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy { some: 3.0 }").unwrap(), &mut bus).unwrap(), 6.0);
}

#[test]
fn test_map_with_default() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy({
            some: (f32),
            thing: (f32) | 4.0,
        }) {} f32 => { Ok(some*thing) }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy { some: 3.0 }").unwrap(), &mut bus).unwrap(), 12.0);
}

#[test]
fn test_map_with_optional() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy({
            some: (f32),
            thing: (f32) optional,
        }) {} f32 => {
            assert_eq!(thing, None);
            Ok(some*2.0)
        }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy { some: 3.0 }").unwrap(), &mut bus).unwrap(), 6.0);
}

#[test]
fn test_arr() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy(some: [f32]) {} f32 => { Ok(some[0]*some[1]*2.0) }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy [2.0, 3.0]").unwrap(), &mut bus).unwrap(), 12.0);
}

#[test]
fn test_map_with_array() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy({
            some: [f32],
        }) {} f32 => { Ok(some[0]*2.0) }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy { some: [3.0] }").unwrap(), &mut bus).unwrap(), 6.0);
}

#[test]
fn test_map_as_whole() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy(some: {f32}) {} f32 => { Ok(some.get("a").unwrap()*some.get("b").unwrap()*2.0) }
    );
    assert_eq!(runtime.translate::<f32>(&Pon::from_string("testy { a: 3, b: 4 }").unwrap(), &mut bus).unwrap(), 24.0);
}

#[test]
fn test_map_missing_field() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy({
            some: (f32),
        }) {} f32 => { Ok(some*2.0) }
    );
    assert!(runtime.translate::<f32>(&Pon::from_string("testy {}").unwrap(), &mut bus).is_err());
}

#[test]
fn test_enum() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    pon_register_functions!(runtime =>
        testy(some: ( enum {
            "hej" => "hello".to_string(),
            "va" => "what".to_string(),
        })) {} String => { Ok(some) }
    );
    assert_eq!(runtime.translate::<String>(&Pon::from_string("testy 'va'").unwrap(), &mut bus).unwrap(), "what".to_string());
}

#[test]
fn test_nil() {
    let mut bus = Bus::new();
    let mut runtime = PonRuntime::new();
    assert_eq!(runtime.translate::<()>(&Pon::from_string("()").unwrap(), &mut bus).unwrap(), ());
}
