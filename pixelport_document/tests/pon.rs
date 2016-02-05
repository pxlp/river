#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;

use std::collections::HashMap;

#[test]
fn test_float() {
    let v = Pon::from_string("5.0");
    assert_eq!(v, Ok(Pon::Number(5.0)));
}

#[test]
fn test_neg_float() {
    let v = Pon::from_string("-5.0");
    assert_eq!(v, Ok(Pon::Number(-5.0)));
}

#[test]
fn test_float_empty_space() {
    let v = Pon::from_string(" 5.0 ");
    assert_eq!(v, Ok(Pon::Number(5.0)));
}

#[test]
fn test_integer() {
    let v = Pon::from_string("5");
    assert_eq!(v, Ok(Pon::Number(5.0)));
}

#[test]
fn test_string() {
    let v = Pon::from_string("'hi'");
    assert_eq!(v, Ok(Pon::String("hi".to_string())));
}

#[test]
fn test_string_escaped() {
    let v = Pon::from_string("'hi\nthis: \\' should work \\\\.'");
    assert_eq!(v, Ok(Pon::String("hi\nthis: ' should work \\.".to_string())));
}

#[test]
fn test_comment_in_string() {
    let v = Pon::from_string("'not a comment: //'");
    assert_eq!(v, Ok(Pon::String("not a comment: //".to_string())));
}

#[test]
fn test_empty_object() {
    let v = Pon::from_string("{}");
    assert_eq!(v, Ok(Pon::Object(HashMap::new())));
}

#[test]
fn test_object_one() {
    let v = Pon::from_string("{ lol: 5.0 }");
    assert_eq!(v, Ok(Pon::Object(hashmap!{
        "lol".to_string() => Pon::Number(5.0)
    })));
}

#[test]
fn test_object_two() {
    let v = Pon::from_string("{ lol: 5.0, hey: 1.1 }");
    assert_eq!(v, Ok(Pon::Object(hashmap!{
        "lol".to_string() => Pon::Number(5.0),
        "hey".to_string() => Pon::Number(1.1)
    })));
}

#[test]
fn test_object_trailing_comma() {
    let v = Pon::from_string("{ lol: 5.0, }");
    assert_eq!(v, Ok(Pon::Object(hashmap!{
        "lol".to_string() => Pon::Number(5.0)
    })));
}

#[test]
fn test_object_complex() {
    let v = Pon::from_string("{ a: [0.0, 0.5], b: [0] }");
    assert_eq!(v, Ok(Pon::Object(hashmap!{
        "a".to_string() => Pon::Array(vec![Pon::Number(0.0), Pon::Number(0.5)]),
        "b".to_string() => Pon::Array(vec![Pon::Number(0.0)])
    })));
}

#[test]
fn test_array_empty() {
    let v = Pon::from_string("[]");
    assert_eq!(v, Ok(Pon::Array(vec![])));
}

#[test]
fn test_array_one() {
    let v = Pon::from_string("[5.0]");
    assert_eq!(v, Ok(Pon::Array(vec![Pon::Number(5.0)])));
}

#[test]
fn test_array_two() {
    let v = Pon::from_string("[5.0, 3.31]");
    assert_eq!(v, Ok(Pon::Array(vec![Pon::Number(5.0), Pon::Number(3.31)])));
}

#[test]
fn test_array_trailing_comma() {
    let v = Pon::from_string("[5.0,]");
    assert_eq!(v, Ok(Pon::Array(vec![Pon::Number(5.0)])));
}

#[test]
fn test_transform_nil() {
    let v = Pon::from_string("static_mesh ()");
    assert_eq!(v, Ok(Pon::PonCall(Box::new(PonCall { function_name: "static_mesh".to_string(), arg: Pon::Nil }))));
}

#[test]
fn test_transform_arg() {
    let v = Pon::from_string("static_mesh { vertices: [0.0, -0.5], indices: [0, 1] }");
    let mut hm = HashMap::new();
    hm.insert("vertices".to_string(), Pon::Array(vec![Pon::Number(0.0), Pon::Number(-0.5)]));
    hm.insert("indices".to_string(),  Pon::Array(vec![Pon::Number(0.0), Pon::Number(1.0)]));
    assert_eq!(v, Ok(Pon::PonCall(Box::new(PonCall { function_name: "static_mesh".to_string(), arg: Pon::Object(hm) }))));
}

#[test]
fn test_transform_number() {
    let v = Pon::from_string("static_mesh 5.0");
    assert_eq!(v, Ok(Pon::PonCall(Box::new(PonCall { function_name: "static_mesh".to_string(), arg: Pon::Number(5.0) }))));
}

#[test]
fn test_dependency_reference() {
    let v = Pon::from_string("@some.test");
    assert_eq!(v, Ok(Pon::DependencyReference(NamedPropRef::new(Selector::root_search("some".to_string()), "test"), None)));
}

#[test]
fn test_reference() {
    let v = Pon::from_string("some.test");
    assert_eq!(v, Ok(Pon::Reference(NamedPropRef::new(Selector::root_search("some".to_string()), "test"))));
}

#[test]
fn test_prop_selector() {
    let v = Pon::from_string("some:[name=else].test");
    let sel = Selector { root: SelectorRoot::Root, path: vec![
        SelectorPath::Search(EntityMatch::Name("some".to_string())),
        SelectorPath::Search(EntityMatch::Name("else".to_string())),
    ] };
    assert_eq!(v, Ok(Pon::Reference(NamedPropRef::new(sel, "test"))));
}

#[test]
fn test_selector_from_string() {
    let v = Selector::from_string("some:[name=else]");
    let sel = Selector { root: SelectorRoot::Root, path: vec![
        SelectorPath::Search(EntityMatch::Name("some".to_string())),
        SelectorPath::Search(EntityMatch::Name("else".to_string())),
    ] };
    assert_eq!(v, Ok(sel));
}

#[test]
fn test_selector_id() {
    let v = Selector::from_string("#567");
    assert_eq!(v, Ok(Selector::id(567)));
}

#[test]
fn test_selector() {
    let v = Pon::from_string("root:[name=hello]");
    assert_eq!(v, Ok(Pon::Selector(Selector::root_search("hello".to_string()))));
}


#[test]
fn test_multiline() {
    let v = Pon::from_string("{
        }");
    assert_eq!(v, Ok(Pon::Object(HashMap::new())));
}

#[test]
fn test_comment() {
    let v = Pon::from_string("
        // Ignore this
        5.0 // And this
    ");
    assert_eq!(v, Ok(Pon::Number(5.0)));
}
