#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;

fn test_doc() -> (EntityId, EntityId, EntityId, EntityId, EntityId, EntityId, Document) {
    let doc = Document::from_string(r#"
        <Root>
            <Entity name="a">
                <Entity name="b" x="5">
                    <Car name="c" />
                </Entity>
                <Entity name="d" />
            </Entity>
            <Entity name="e" x="5" />
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
fn test_selection() {
    let (root, a, b, c, d, e, doc) = test_doc();
    let selector = Selector::from_string("this:[x=5]").unwrap();
    let mut selection = Selection::new(selector, a);
    let change = selection.init(&doc);
    assert_eq!(change, SelectionChange { added: vec![b], removed: vec![] });
}


#[test]
fn test_selection_remove() {
    let (root, a, b, c, d, e, mut doc) = test_doc();
    doc.close_cycle();
    let selector = Selector::from_string("this:[x=5]").unwrap();
    let mut selection = Selection::new(selector, a);
    let change = selection.init(&doc);
    assert_eq!(change, SelectionChange { added: vec![b], removed: vec![] });
    doc.remove_entity(a);
    let cycle_changes = doc.close_cycle();
    let change = selection.cycle(&doc, &cycle_changes);
    assert_eq!(change, SelectionChange { added: vec![], removed: vec![b] });
}


#[test]
fn test_selection_add_and_set_property() {
    let (root, a, b, c, d, e, mut doc) = test_doc();
    doc.close_cycle();
    let selector = Selector::from_string("root:[z=2]").unwrap();
    let mut selection = Selection::new(selector, a);
    let change = selection.init(&doc);
    assert_eq!(change, SelectionChange { added: vec![], removed: vec![] });
    let z = doc.append_entity(None, Some(c), "Test", None).unwrap();
    doc.set_property(z, "z", Pon::Number(2.0), false);
    let cycle_changes = doc.close_cycle();
    println!("cc {:?}", cycle_changes);
    let change = selection.cycle(&doc, &cycle_changes);
    assert_eq!(change, SelectionChange { added: vec![z], removed: vec![] });
}
