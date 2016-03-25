use channel::*;
use selection::*;
use topic::*;
use document::*;
use pon::*;
use std::collections::HashMap;

use regex::Regex;


pub struct DocStream {
    pub channel_id: ChannelId,
    pub client_id: ClientId,
    pub selection: Selection,
    pub property_regex: Option<Regex>,
    pub topic: Topic
}
impl DocStream {
    pub fn init(&mut self, doc: &Document) -> Option<OutgoingMessage> {
        let change = self.selection.init(doc);
        let (added, removed) = self.handle_entities_changed(doc, change);
        let properties: Vec<PropRef> = {
            if let &Some(ref property_regex) = &self.property_regex {
                added.iter()
                    .flat_map(|entity| doc.get_properties(entity.entity_id).unwrap_or(Vec::new()))
                    .filter(|pr| {
                        self.selection.contains(pr.entity_id) &&
                        property_regex.is_match(&pr.property_key)
                    }).collect()
            } else {
                Vec::new()
            }
        };
        let updated_properties: Vec<DocStreamPropertyValue> = properties.iter().map(|pr| {
                DocStreamPropertyValue {
                    entity_id: pr.entity_id,
                    property_key: pr.property_key.clone(),
                    property_expression: match doc.get_property_expression(&pr) {
                        Ok(v) => Some(v.clone()),
                        Err(_) => None
                    },
                    property_value: match doc.get_property_raw(pr.entity_id, &pr.property_key) {
                        Ok(v) => Ok(format!("{:?}", v)),
                        Err(err) => Err(err.to_string())
                    }
                }
            }).collect();
        if added.len() > 0 || removed.len() > 0 {
            Some(OutgoingMessage {
                channel_id: self.channel_id.clone(),
                client_id: self.client_id.clone(),
                message: Ok(Box::new(DocStreamCycle {
                    entities_added: added,
                    entities_removed: removed,
                    updated_properties: updated_properties
                }))
            })
        } else {
            None
        }
    }
    pub fn on_cycle(&mut self, doc: &mut Document, changes: &CycleChanges) -> Option<OutgoingMessage> {
        let sel_change = self.selection.cycle(doc, changes);
        let (added, removed) = self.handle_entities_changed(doc, sel_change);

        // Update properties
        let updated_properties: Vec<DocStreamPropertyValue> = if let &Some(ref property_regex) = &self.property_regex {
            let selection = &self.selection;
            self.topic.invalidated(&doc.bus, &changes.invalidations_log, |pr| {
                selection.contains(pr.entity_id) &&
                property_regex.is_match(&pr.property_key)
            }).into_iter().map(|pr: PropRef| {
                DocStreamPropertyValue {
                    entity_id: pr.entity_id,
                    property_key: pr.property_key.clone(),
                    property_expression: match doc.get_property_expression(&pr) {
                        Ok(v) => Some(v.clone()),
                        Err(_) => None
                    },
                    property_value: match doc.get_property_raw(pr.entity_id, &pr.property_key) {
                        Ok(v) => Ok(format!("{:?}", v)),
                        Err(err) => Err(err.to_string())
                    }
                }
            }).collect()
        } else {
            Vec::new()
        };
        if added.len() > 0 || removed.len() > 0 || updated_properties.len() > 0 {
            Some(OutgoingMessage {
                channel_id: self.channel_id.clone(),
                client_id: self.client_id.clone(),
                message: Ok(Box::new(DocStreamCycle {
                    entities_added: added,
                    entities_removed: removed,
                    updated_properties: updated_properties
                }))
            })
        } else {
            None
        }
    }
    fn handle_entities_changed(&self, document: &Document, change: SelectionChange) -> (Vec<DocStreamAddedEntity>, Vec<EntityId>) {
        let mut added = vec![];
        for entity_id in change.added {
            added.push(DocStreamAddedEntity {
                entity_id: entity_id,
                type_name: document.get_entity_type_name(entity_id).unwrap(),
                parent_id: document.get_parent(entity_id).unwrap()
            });
        }
        (added, change.removed)
    }
}


#[derive(Debug, PartialEq, Clone)]
pub struct DocStreamCycle {
    pub entities_added: Vec<DocStreamAddedEntity>,
    pub entities_removed: Vec<EntityId>,
    pub updated_properties: Vec<DocStreamPropertyValue>
}
impl ToPon for DocStreamCycle {
    fn to_pon(&self) -> Pon {
        Pon::call("doc_stream_cycle", Pon::Object(hashmap![
            "entities_added" => Pon::Array(self.entities_added.iter().map(|x| x.to_pon()).collect()),
            "entities_removed" => Pon::Array(self.entities_removed.iter().map(|x| x.to_pon()).collect()),
            "updated_properties" => Pon::Array(self.updated_properties.iter().map(|x| x.to_pon()).collect())
        ]))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DocStreamAddedEntity {
    pub entity_id: EntityId,
    pub parent_id: Option<EntityId>,
    pub type_name: String
}
impl ToPon for DocStreamAddedEntity {
    fn to_pon(&self) -> Pon {
        let mut hm = HashMap::new();
        hm.insert("entity_id".to_string(), self.entity_id.to_pon());
        if let Some(parent_id) = self.parent_id {
            hm.insert("parent_id".to_string(), parent_id.to_pon());
        }
        hm.insert("type_name".to_string(), self.type_name.to_pon());
        Pon::Object(hm)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DocStreamPropertyValue {
    pub entity_id: EntityId,
    pub property_key: String,
    pub property_expression: Option<Pon>,
    pub property_value: Result<String, String>
}
impl ToPon for DocStreamPropertyValue {
    fn to_pon(&self) -> Pon {
        let mut hm = HashMap::new();
        hm.insert("entity_id".to_string(), self.entity_id.to_pon());
        hm.insert("property_key".to_string(), self.property_key.to_pon());
        if let &Some(ref pe) = &self.property_expression {
            hm.insert("property_expression".to_string(), pe.to_pon());
        }
        match &self.property_value {
            &Ok(ref v) => { hm.insert("property_value".to_string(), v.to_pon()); },
            &Err(ref v) => { hm.insert("property_error".to_string(), v.to_pon()); },
        }
        Pon::Object(hm)
    }
}
