
use selector::*;
use std::collections::HashSet;
use document::*;
use pon::*;
use std::collections::hash_set::Iter;

#[derive(Debug)]
pub struct Selection {
    pub selector: Selector,
    pub from_entity_id: EntityId,
    in_selection: HashSet<EntityId>
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SelectionChange {
    pub added: Vec<EntityId>,
    pub removed: Vec<EntityId>,
}
impl SelectionChange {
    pub fn changed(&self) -> bool {
        self.added.len() > 0 || self.removed.len() > 0
    }
}

impl Selection {
    pub fn new(selector: Selector, from_entity_id: EntityId) -> Selection {
        Selection {
            selector: selector,
            from_entity_id: from_entity_id,
            in_selection: HashSet::new()
        }
    }
    pub fn init(&mut self, document: &Document) -> SelectionChange {
        self.reevaluate_all(document)
    }
    pub fn cycle(&mut self, document: &Document, changes: &CycleChanges) -> SelectionChange {
        for pr in &changes.set_properties {
            if self.selector.property_of_interest(&pr.property_key) {
                return self.reevaluate_all(document);
            }
        }
        let mut sel_changes = SelectionChange {
            added: Vec::new(),
            removed: Vec::new()
        };
        for entity_id in &changes.entities_added {
            if self.selector.matches(document, self.from_entity_id, *entity_id) {
                self.in_selection.insert(*entity_id);
                sel_changes.added.push(*entity_id);
            }
        }
        for entity in &changes.entities_removed {
            if self.in_selection.remove(&entity.id) {
                sel_changes.removed.push(entity.id);
            }
        }
        sel_changes
    }
    pub fn contains(&self, entity_id: EntityId) -> bool {
        self.in_selection.contains(&entity_id)
    }
    pub fn iter(&self) -> Iter<EntityId> {
        self.in_selection.iter()
    }
    fn reevaluate_all(&mut self, document: &Document) -> SelectionChange {
        let all_entities: Vec<EntityId> = { document.entities_iter().map(|x| x.clone()).collect() };
        let mut in_selection = HashSet::new();
        for entity_id in all_entities {
            if self.selector.matches(document, self.from_entity_id, entity_id) {
                in_selection.insert(entity_id);
            }
        }
        let added = in_selection.difference(&self.in_selection).cloned().collect();
        let removed = self.in_selection.difference(&in_selection).cloned().collect();
        self.in_selection = in_selection;
        SelectionChange {
            added: added,
            removed: removed
        }
    }
}
