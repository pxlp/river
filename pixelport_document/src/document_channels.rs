

use pon::*;
use document::*;
use selector::*;
use pon_translater::*;
use std::collections::HashMap;
use bus::*;
use doc_stream::*;
use channel::*;
use selection::*;
use topic::*;
use regex::*;


#[derive(Debug, PartialEq, Clone)]
pub struct SetPropertiesRequest {
    pub entity: Selector,
    pub properties: HashMap<String, Pon>
}

#[derive(Debug, PartialEq, Clone)]
pub struct AppendEntityRequest {
    pub entity_id: Option<u64>,
    pub parent: Selector,
    pub type_name: String,
    pub properties: HashMap<String, Pon>
}

#[derive(Debug, PartialEq, Clone)]
pub struct RemoveEntityRequest {
    pub entity: Selector
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClearChildrenRequest {
    pub entity: Selector
}

#[derive(Debug, PartialEq, Clone)]
pub struct ReserveEntityIdsRequest {
    pub count: u64
}


#[derive(Debug, PartialEq, Clone)]
pub struct DocStreamCreateRequest {
    pub channel_id: String,
    pub selector: Selector,
    pub property_regex: Option<String>
}

#[derive(Debug, PartialEq, Clone)]
pub struct DocStreamDestroyRequest {
    pub channel_id: String
}


pub fn pon_document_requests(translater: &mut PonTranslater) {
    pon_register_functions!("Document", translater =>

        r#"Set properties of an entity. Dependencies and functions in `properties` are not
        evaluated at call time.

        For instance, in `set_properties { entity: root, properties: { x: @root.y } }` the
        `@root.y` will not be evaluated at request time, but rather set up as a dependency in the
        document."#,
        set_properties({
            entity: (Selector),
            properties: {Pon},
        }) SetPropertiesRequest => {
            Ok(SetPropertiesRequest {
                entity: entity,
                properties: properties
            })
        }

        r#"Append an entity to a parent entity. Properties are not evaluted at request time (see
        set_properties for details)"#,
        append_entity({
            entity_id: (f32) optional,
            parent: (Selector),
            type_name: (String),
            properties: {Pon},
        }) AppendEntityRequest => {
            Ok(AppendEntityRequest {
                entity_id: match entity_id {
                    Some(v) => Some(v as EntityId),
                    None => None
                },
                parent: parent,
                type_name: type_name,
                properties: properties
            })
        }

        "Remove an entity.",
        remove_entity({
            entity: (Selector),
        }) RemoveEntityRequest => {
            Ok(RemoveEntityRequest {
                entity: entity
            })
        }

        "Clear children of an entity.",
        clear_children({
            entity: (Selector),
        }) ClearChildrenRequest => {
            Ok(ClearChildrenRequest {
                entity: entity
            })
        }


        "Reserve a number of entity ids, that can then be used in append_entity.",
        reserve_entity_ids({
            count: (f32),
        }) ReserveEntityIdsRequest => {
            Ok(ReserveEntityIdsRequest {
                count: count as u64
            })
        }

        r#"Create a doc stream. Streams changes to the document, filtered by `selector` and
        optionally `property_regex`."#,
        doc_stream_create({
            channel_id: (String),
            selector: (Selector),
            property_regex: (String) optional,
        }) DocStreamCreateRequest => {
            Ok(DocStreamCreateRequest {
                channel_id: channel_id,
                selector: selector,
                property_regex: property_regex,
            })
        }

        r#"Create a doc stream. Streams changes to the document, filtered by `selector` and
        optionally `property_regex`."#,
        doc_stream_destroy({
            channel_id: (String),
        }) DocStreamDestroyRequest => {
            Ok(DocStreamDestroyRequest {
                channel_id: channel_id,
            })
        }
    );
}


macro_rules! try_find_first {
    ($inc:expr, $selector:expr, $doc:expr, $root_id:expr) => (match $selector.find_first($doc, $root_id) {
        Ok(val) => val,
        Err(err) => return vec![$inc.bad_request(&format!("No such entity: {}", $selector.to_string()))]
    })
}

pub struct DocumentChannels {
    doc_streams: HashMap<(ClientId, ChannelId), DocStream>
}

impl DocumentChannels {
    pub fn new() -> DocumentChannels {
        DocumentChannels {
            doc_streams: HashMap::new()
        }
    }
    pub fn cycle_changes(&mut self, doc: &mut Document, changes: &CycleChanges) -> Vec<OutgoingMessage> {
        let mut messages = Vec::new();
        for (_, doc_stream) in self.doc_streams.iter_mut() {
            if let Some(message) = doc_stream.on_cycle(doc, changes) {
                messages.push(message);
            }
        }
        messages
    }
    pub fn handle_request(&mut self, inc: &IncomingMessage, doc: &mut Document) -> Vec<OutgoingMessage> {
        if let Some(set_properties) = (*inc.message).downcast_ref::<SetPropertiesRequest>() {
            let root_id = doc.get_root().expect("Document missing root");
            let ent = try_find_first!(inc, set_properties.entity, doc, root_id);
            for (key, pon) in &set_properties.properties {
                if let Err(err) = doc.set_property(ent, &key, pon.clone(), false) {
                    warn!("Failed to set property {} {}, error: {:?}", key, pon.to_string(), err);
                }
            }
            return vec![inc.ok(())];
        }
        if let Some(append_entity) = (*inc.message).downcast_ref::<AppendEntityRequest>() {
            let root_id = doc.get_root().expect("AppendEntity Document missing root");
            let parent_id = try_find_first!(inc, append_entity.parent, doc, root_id);
            let ent = doc.append_entity(append_entity.entity_id, Some(parent_id), &append_entity.type_name, None).expect("AppendEntity failed to append entity");
            for (key, pon) in &append_entity.properties {
                if let Err(err) = doc.set_property(ent, &key, pon.clone(), false) {
                    warn!("AppendEntity Failed to set property {} {}, error: {:?}", key, pon.to_string(), err);
                }
            }
            return vec![inc.ok(ent)];
        }
        if let Some(remove_entity) = (*inc.message).downcast_ref::<RemoveEntityRequest>() {
            let root_id = doc.get_root().expect("RemoveEntity Document missing root");
            let entity_id = try_find_first!(inc, remove_entity.entity, doc, root_id);
            return vec![match doc.remove_entity(entity_id) {
                Ok(()) => inc.ok(()),
                Err(err) => inc.bad_request(&format!("Failed to remove entity {}: {:?}", remove_entity.entity.to_string(), err))
            }];
        }
        if let Some(clear_children) = (*inc.message).downcast_ref::<ClearChildrenRequest>() {
            let root_id = doc.get_root().expect("ClearChildren Document missing root");
            let entity_id = try_find_first!(inc, clear_children.entity, doc, root_id);
            return vec![match doc.clear_children(entity_id) {
                Ok(()) => inc.ok(()),
                Err(err) => inc.bad_request(&format!("ClearChildren failed to clear children of {}: {:?}", clear_children.entity.to_string(), err))
            }];
        }
        if let Some(reserve_entity_ids) = (*inc.message).downcast_ref::<ReserveEntityIdsRequest>() {
            let res = doc.reserve_entity_ids(reserve_entity_ids.count);
            return vec![inc.ok(vec![res.min, res.max])]
        }
        if let Some(doc_stream_create) = (*inc.message).downcast_ref::<DocStreamCreateRequest>() {
            let root_id = doc.get_root().expect("Document missing root");
            let selection = Selection::new(doc_stream_create.selector.clone(), root_id);
            let mut doc_stream = DocStream {
                channel_id: doc_stream_create.channel_id.clone(),
                client_id: inc.client_id.clone(),
                selection: selection,
                property_regex: match &doc_stream_create.property_regex {
                    &Some(ref regex) => Some(Regex::new(regex).expect("Non parseable regex")),
                    &None => None
                },
                topic: Topic::new()
            };
            let mut messages = Vec::new();
            if let Some(message) = doc_stream.init(doc) {
                messages.push(message);
            }
            self.doc_streams.insert((inc.client_id.clone(), doc_stream_create.channel_id.clone()), doc_stream);
            messages.push(inc.ok(()));
            return messages;
        }
        if let Some(doc_stream_destroy) = (*inc.message).downcast_ref::<DocStreamDestroyRequest>() {
            self.doc_streams.remove(&(inc.client_id.clone(), doc_stream_destroy.channel_id.clone()));
            return vec![inc.ok(())];
        }
        Vec::new()
    }
}
