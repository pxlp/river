#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;

fn test_doc() -> (EntityId, EntityId, EntityId, EntityId, EntityId, EntityId, Document) {
    let doc = Document::from_string(r#"
        <Root>
            <Entity name="a">
                <Entity name="b" x="5" y="1">
                    <Car name="c" />
                </Entity>
                <Entity name="d" y="3" />
            </Entity>
            <Entity name="e" x="5" y="3" />
        </Entity>
    </Root>"#).unwrap();
    (
        doc.get_root().unwrap(),
        doc.get_entity_by_name("a").unwrap(),
        doc.get_entity_by_name("b").unwrap(),
        doc.get_entity_by_name("c").unwrap(),
        doc.get_entity_by_name("d").unwrap(),
        doc.get_entity_by_name("e").unwrap(),
        doc
    )
}

#[test]
fn test_selector_find_first_root_search_name() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:[name=d]").unwrap();
    assert_eq!(selector.find_first(&doc, root), Ok(d));
}

#[test]
fn test_selector_matches_root_search_property() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:[x=5]").unwrap();
    assert!(!selector.matches(&doc, root, a));
    assert!(selector.matches(&doc, root, b));
    assert!(!selector.matches(&doc, root, c));
    assert!(!selector.matches(&doc, root, d));
    assert!(selector.matches(&doc, root, e));
    assert!(selector.property_of_interest("x"));
}

#[test]
fn test_selector_matches_root_search_property_and() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:[[x=5] && [y=3]]").unwrap();
    assert!(!selector.matches(&doc, root, a));
    assert!(!selector.matches(&doc, root, b));
    assert!(!selector.matches(&doc, root, c));
    assert!(!selector.matches(&doc, root, d));
    assert!(selector.matches(&doc, root, e));
    assert!(selector.property_of_interest("x"));
}

#[test]
fn test_selector_matches_root_search_property_or() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:[[x=5] || [y=3]]").unwrap();
    assert!(!selector.matches(&doc, root, a));
    assert!(selector.matches(&doc, root, b));
    assert!(!selector.matches(&doc, root, c));
    assert!(selector.matches(&doc, root, d));
    assert!(selector.matches(&doc, root, e));
    assert!(selector.property_of_interest("x"));
}


#[test]
fn test_selector_matches_root_search_property_not_equals() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:[x!=5]").unwrap();
    assert!(selector.matches(&doc, root, a));
    assert!(!selector.matches(&doc, root, b));
    assert!(selector.matches(&doc, root, c));
    assert!(selector.matches(&doc, root, d));
    assert!(!selector.matches(&doc, root, e));
    assert!(selector.property_of_interest("x"));
}


#[test]
fn test_selector_matches_root_search_inv_property() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:![x=5]").unwrap();
    assert!(selector.matches(&doc, root, a));
    assert!(!selector.matches(&doc, root, b));
    assert!(!selector.matches(&doc, root, c));
    assert!(selector.matches(&doc, root, d));
    assert!(!selector.matches(&doc, root, e));
}

#[test]
fn test_selector_matches_root_search_inv_property_then_any() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:![x=5]:*").unwrap();
    assert!(selector.matches(&doc, a, a));
    assert!(!selector.matches(&doc, a, b));
    assert!(!selector.matches(&doc, a, c));
    assert!(selector.matches(&doc, a, d));
    assert!(!selector.matches(&doc, a, e));
}


#[test]
fn test_selector_matches_this_search_inv_property() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:![x=5]").unwrap();
    assert!(selector.matches(&doc, a, a));
    assert!(!selector.matches(&doc, a, b));
    assert!(!selector.matches(&doc, a, c));
    assert!(selector.matches(&doc, a, d));
    assert!(!selector.matches(&doc, a, e));
}

#[test]
fn test_selector_matches_this_search_inv_property_then_any() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:![x=5]:*").unwrap();
    assert!(selector.matches(&doc, a, a));
    assert!(!selector.matches(&doc, a, b));
    assert!(!selector.matches(&doc, a, c));
    assert!(selector.matches(&doc, a, d));
    assert!(!selector.matches(&doc, a, e));
}

#[test]
fn test_selector_matches_root_property_exists() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:[x]").unwrap();
    assert!(!selector.matches(&doc, a, a));
    assert!(selector.matches(&doc, a, b));
    assert!(!selector.matches(&doc, a, c));
    assert!(!selector.matches(&doc, a, d));
    assert!(selector.matches(&doc, a, e));
    assert!(selector.property_of_interest("x"));
}


#[test]
fn test_selector_matches_this_child() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this/*").unwrap();
    assert!(!selector.matches(&doc, a, a));
    assert!(selector.matches(&doc, a, b));
    assert!(!selector.matches(&doc, a, c));
    assert!(selector.matches(&doc, a, d));
    assert!(!selector.matches(&doc, a, e));
}

#[test]
fn test_selector_find_first_this_search_property() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:[x=5]").unwrap();
    assert_eq!(selector.find_first(&doc, a), Ok(b));
}

#[test]
fn test_selector_find_first_this_search_inv_property() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:![x=5]").unwrap();
    assert_eq!(selector.find_first(&doc, a), Ok(d));
}

#[test]
fn test_selector_matches_this_search_property_then_any() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:[x=5]:*").unwrap();
    assert!(!selector.matches(&doc, a, a));
    assert!(selector.matches(&doc, a, b));
    assert!(selector.matches(&doc, a, c));
    assert!(!selector.matches(&doc, a, d));
    assert!(!selector.matches(&doc, a, e));
}

#[test]
fn test_selector_matches_root_search_property_then_any() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("root:[x=5]:*").unwrap();
    assert!(!selector.matches(&doc, a, a));
    assert!(selector.matches(&doc, a, b));
    assert!(selector.matches(&doc, a, c));
    assert!(!selector.matches(&doc, a, d));
    assert!(selector.matches(&doc, a, e));
}

#[test]
fn test_selector_matches_this_search_any() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:*").unwrap();
    assert!(selector.matches(&doc, a, a));
    assert!(selector.matches(&doc, a, b));
    assert!(selector.matches(&doc, a, c));
    assert!(selector.matches(&doc, a, d));
    assert!(!selector.matches(&doc, a, e));
}

#[test]
fn test_selector_find_first_this_prev_sibling() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this|prev-sibling|").unwrap();
    assert_eq!(selector.find_first(&doc, d), Ok(b));
}

#[test]
fn test_selector_find_first_next_sibling() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this|next-sibling|").unwrap();
    assert_eq!(selector.find_first(&doc, b), Ok(d));
}


#[test]
fn test_selector_matches_this_search_type_name() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:Car").unwrap();
    assert!(!selector.matches(&doc, a, a));
    assert!(!selector.matches(&doc, a, b));
    assert!(selector.matches(&doc, a, c));
    assert!(!selector.matches(&doc, a, d));
    assert!(!selector.matches(&doc, a, e));
}


#[test]
fn test_selector_matches_this_parent() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this|parent|").unwrap();
    assert!(selector.matches(&doc, b, a));
    assert!(!selector.matches(&doc, b, b));
    assert!(!selector.matches(&doc, b, c));
    assert!(!selector.matches(&doc, b, d));
    assert!(!selector.matches(&doc, b, e));
}
