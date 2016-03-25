
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

pub type SocketToken = usize;



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

pub fn pon_document_requests(translater: &mut PonTranslater) {
    pon_register_functions!("Document", translater =>

        "",
        set_properties({
            entity: (Selector),
            properties: {Pon},
        }) SetPropertiesRequest => {
            Ok(SetPropertiesRequest {
                entity: entity,
                properties: properties
            })
        }

        "",
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

    );
}

pub fn document_handle_request(request: Box<BusValue>, socket_token: SocketToken, doc: &mut Document) -> Option<Result<Box<OutMessage>, RequestError>> {
    let request = match request.downcast::<SetPropertiesRequest>() {
        Ok(set_properties) => {
            let root_id = doc.get_root().expect("Document missing root");
            let ent = match set_properties.entity.find_first(doc, root_id) {
                Ok(ent) => ent,
                Err(_) => return Some(Err(RequestError {
                    request_id: "".to_string(),
                    error_type: RequestErrorType::BadRequest,
                    message: format!("No such entity: {}", set_properties.entity.to_string())
                }))
            };
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
            let parent_id = match append_entity.parent.find_first(doc, root_id) {
                Ok(ent) => ent,
                Err(_) => return Some(Err(RequestError {
                    request_id: "".to_string(),
                    error_type: RequestErrorType::BadRequest,
                    message: format!("No such parent entity: {}", append_entity.parent.to_string())
                }))
            };
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
    None
}
