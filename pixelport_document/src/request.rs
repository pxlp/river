
use std::fmt::Debug;
use pon::*;
use document::*;
use selector::*;
use pon_translater::*;
use std::collections::HashMap;
use bus::*;

pub trait OutMessage: ToPon + Send + Debug {
}
impl<T: ToPon + Send + Debug> OutMessage for T {}

#[derive(Debug)]
pub struct RequestError {
    pub request_id: String,
    pub error_type: RequestErrorType,
    pub message: String
}

#[derive(Debug)]
pub enum RequestErrorType {
    BadRequest,
    InternalError,
}

pub type SocketToken = usize; // This is either a tcp socket or a c process

#[derive(Debug)]
pub struct OutgoingMessage {
    pub channel_id: String, // Either a request_id or a stream_id
    pub socket_token: SocketToken,
    pub message: Result<Box<OutMessage>, RequestError>
}
impl OutgoingMessage {
    pub fn to_tcpmessage(&self) -> String {
        match &self.message {
            &Ok(ref message) => format!("{} ok {}", self.channel_id, message.to_pon().to_string()),
            &Err(ref err) => format!("{} err {:?}", self.channel_id, err),
        }
    }
}



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

    );
}


macro_rules! try_find_first {
    ($selector:expr, $doc:expr, $root_id:expr) => (match $selector.find_first($doc, $root_id) {
        Ok(val) => val,
        Err(err) => {
            return Some(Err(RequestError {
                request_id: "".to_string(),
                error_type: RequestErrorType::BadRequest,
                message: format!("No such entity: {}", $selector.to_string())
            }));
        }
    })
}
pub fn document_handle_request(request: Box<BusValue>, socket_token: SocketToken, doc: &mut Document) -> Option<Result<Box<OutMessage>, RequestError>> {
    let request = match request.downcast::<SetPropertiesRequest>() {
        Ok(set_properties) => {
            let root_id = doc.get_root().expect("Document missing root");
            let ent = try_find_first!(set_properties.entity, doc, root_id);
            for (key, pon) in set_properties.properties {
                if let Err(err) = doc.set_property(ent, &key, pon.clone(), false) {
                    warn!("Failed to set property {} {}, error: {:?}", key, pon.to_string(), err);
                }
            }
            return Some(Ok(Box::new(())));
        },
        Err(request) => request
    };
    let request = match request.downcast::<AppendEntityRequest>() {
        Ok(append_entity) => {
            let root_id = doc.get_root().expect("AppendEntity Document missing root");
            let parent_id = try_find_first!(append_entity.parent, doc, root_id);
            let ent = doc.append_entity(append_entity.entity_id, Some(parent_id), &append_entity.type_name, None).expect("AppendEntity failed to append entity");
            for (key, pon) in append_entity.properties {
                if let Err(err) = doc.set_property(ent, &key, pon.clone(), false) {
                    warn!("AppendEntity Failed to set property {} {}, error: {:?}", key, pon.to_string(), err);
                }
            }
            return Some(Ok(Box::new(ent)));
        },
        Err(request) => request
    };
    let request = match request.downcast::<RemoveEntityRequest>() {
        Ok(remove_entity) => {
            let root_id = doc.get_root().expect("RemoveEntity Document missing root");
            let entity_id = try_find_first!(remove_entity.entity, doc, root_id);
            return Some(match doc.remove_entity(entity_id) {
                Ok(()) => Ok(Box::new(())),
                Err(err) =>  Err(RequestError {
                    request_id: "".to_string(),
                    error_type: RequestErrorType::BadRequest,
                    message: format!("RemoveEntity failed to remove entity {}: {:?}", remove_entity.entity.to_string(), err)
                })
            });
        },
        Err(request) => request
    };
    let request = match request.downcast::<ClearChildrenRequest>() {
        Ok(clear_children) => {
            let root_id = doc.get_root().expect("ClearChildren Document missing root");
            let entity_id = try_find_first!(clear_children.entity, doc, root_id);
            return Some(match doc.clear_children(entity_id) {
                Ok(()) => Ok(Box::new(())),
                Err(err) =>  Err(RequestError {
                    request_id: "".to_string(),
                    error_type: RequestErrorType::BadRequest,
                    message: format!("ClearChildren failed to clear children of {}: {:?}", clear_children.entity.to_string(), err)
                })
            });
        },
        Err(request) => request
    };
    let request = match request.downcast::<ReserveEntityIdsRequest>() {
        Ok(reserve_entity_ids) => {
            let res = doc.reserve_entity_ids(reserve_entity_ids.count);
            return Some(Ok(Box::new(vec![res.min, res.max])))
        },
        Err(request) => request
    };
    None
}
