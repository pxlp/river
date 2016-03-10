#[macro_use]
extern crate pixelport_document;

use pixelport_document::*;


macro_rules! assert_eq_unsorted {
    ($a:expr, $b:expr) => ({
        let mut a = $a;
        let mut b = $b;
        a.sort();
        b.sort();
        assert_eq!(a, b);
    })
}

#[test]
fn test_invalidated_prop_straight() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" />"#).unwrap();
    let mut topic = AllTopic::new();
    let ent = document.get_entity_by_name("tmp").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}
#[test]
fn test_invalidated_prop_straight_change_twice() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" />"#).unwrap();
    let mut topic = AllTopic::new();
    let ent = document.get_entity_by_name("tmp").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    document.set_property(ent, "x", Pon::Number(19.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_prop_one_dep() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" y="@this.x" />"#).unwrap();
    let mut topic = AllTopic::new();
    let ent = document.get_entity_by_name("tmp").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_prop_one_dep_doesnt_exist_first() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" y="@this.x" />"#).unwrap();
    let mut topic = AllTopic::new();
    let ent = document.get_entity_by_name("tmp").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "y")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y")]);
    document.set_property(ent, "x", Pon::Number(19.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_prop_two_dep_doesnt_exist_first() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" z="@this.y" />"#).unwrap();
    let mut topic = AllTopic::new();
    let ent = document.get_entity_by_name("tmp").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    document.set_property(ent, "y", Pon::from_string("@this.x").unwrap(), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "y"), PropRef::new(1, "z")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y"), PropRef::new(1, "z")]);
    document.set_property(ent, "x", Pon::Number(19.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y"), PropRef::new(1, "z")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_prop_three_dep_doesnt_exist_first() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" w="@this.z" />"#).unwrap();
    let mut topic = AllTopic::new();
    let ent = document.get_entity_by_name("tmp").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    document.set_property(ent, "y", Pon::from_string("@this.x").unwrap(), false).ok().unwrap();
    document.set_property(ent, "z", Pon::from_string("@this.y").unwrap(), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "y"), PropRef::new(1, "z"), PropRef::new(1, "w")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y"), PropRef::new(1, "z"), PropRef::new(1, "w")]);
    document.set_property(ent, "x", Pon::Number(19.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "y"), PropRef::new(1, "z"), PropRef::new(1, "w")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}
#[test]
fn test_invalidated_prop_parent() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="a" x="5.0"><Entity name="b" y="@parent.x" /></Entity>"#).unwrap();
    let mut topic = AllTopic::new();
    let ent_a = document.get_entity_by_name("a").unwrap();
    let ent_b = document.get_entity_by_name("b").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(ent_a, "x"), PropRef::new(ent_b, "y")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent_a, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(ent_a, "x"), PropRef::new(ent_b, "y")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_prop_one_exists_one_doesnt() {
    let mut document = Document::from_string(PonTranslater::new(), r#"<Entity name="tmp" x="5.0" z="[@this.x, @this.y]" />"#).unwrap();
    let mut topic = AllTopic::new();
    let ent = document.get_entity_by_name("tmp").unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    document.set_property(ent, "y", Pon::from_string("7.0").unwrap(), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "y"), PropRef::new(1, "z")]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "z")]);
    document.set_property(ent, "x", Pon::Number(19.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x"), PropRef::new(1, "z")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_change_twice() {
    let mut document = Document::new(PonTranslater::new());
    let mut topic = AllTopic::new();
    let ent = document.append_entity(None, None, "Tmp", None).unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    document.set_property(ent, "x", Pon::Number(12.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_change_twice_in_one_frame() {
    let mut document = Document::new(PonTranslater::new());
    let mut topic = AllTopic::new();
    let ent = document.append_entity(None, None, "Tmp", None).unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::Number(9.0), false).ok().unwrap();
    document.set_property(ent, "x", Pon::Number(3.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_change_twice_in_one_frame_deps() {
    let mut document = Document::new(PonTranslater::new());
    let mut topic = AllTopic::new();
    let ent = document.append_entity(None, None, "Tmp", None).unwrap();
    document.set_property(ent, "y", Pon::Number(5.0), false).ok().unwrap();
    document.set_property(ent, "z", Pon::Number(6.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::from_string("@this.y").unwrap(), false).ok().unwrap();
    document.set_property(ent, "x", Pon::from_string("@this.z").unwrap(), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}

#[test]
fn test_invalidated_change_twice_deps() {
    let mut document = Document::new(PonTranslater::new());
    let mut topic = AllTopic::new();
    let ent = document.append_entity(None, None, "Tmp", None).unwrap();
    document.set_property(ent, "y", Pon::Number(5.0), false).ok().unwrap();
    document.set_property(ent, "z", Pon::Number(6.0), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
    document.set_property(ent, "x", Pon::from_string("@this.y").unwrap(), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    document.set_property(ent, "x", Pon::from_string("@this.z").unwrap(), false).ok().unwrap();
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![PropRef::new(1, "x")]);
    let cycle_changes = document.close_cycle(); let inv = topic.invalidated(&document.bus, &cycle_changes.invalidations_log);
    assert_eq_unsorted!(inv, vec![]);
}
