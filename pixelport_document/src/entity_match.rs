
use document::*;
use pon::*;
use bus::*;


#[derive(Debug, Clone, PartialEq)]
pub enum EntityMatch {
    Any,
    Name(String),
    TypeName(String),
    PropertyValueEquals { property: String, value: Box<Pon> },
    PropertyValueNotEquals { property: String, value: Box<Pon> },
    PropertyExists(String),
    And(Box<EntityMatch>, Box<EntityMatch>),
    Or(Box<EntityMatch>, Box<EntityMatch>)
}

impl EntityMatch {
    pub fn property_value(property: String, value: Pon) -> EntityMatch {
        if property == "name" {
            EntityMatch::Name(match value {
                 Pon::String(string) => string.to_string(),
                 _ => panic!("Name match must always be a string")
             })
        } else {
            EntityMatch::PropertyValueEquals { property: property, value: Box::new(value) }
        }
    }
    pub fn matches(&self, document: &Document, entity_id: EntityId) -> bool {
        match self {
            &EntityMatch::Any => true,
            &EntityMatch::TypeName(ref name) => match document.get_entity_type_name(entity_id) {
                Ok(val) => &val == name,
                _ => false
            },
            &EntityMatch::Name(ref name) => match document.get_entity_name(entity_id) {
                Ok(&Some(ref val)) => val == name,
                _ => false
            },
            &EntityMatch::PropertyValueEquals { ref property, box ref value } => match document.get_property_raw(entity_id, property) {
                Ok(val) => (*val).bus_value_equals(&document.runtime.translate_raw(value, &document.bus).unwrap()),
                Err(_) => false
            },
            &EntityMatch::PropertyValueNotEquals { ref property, box ref value } => match document.get_property_raw(entity_id, property) {
                Ok(val) => !(*val).bus_value_equals(&document.runtime.translate_raw(value, &document.bus).unwrap()),
                Err(_) => true
            },
            &EntityMatch::PropertyExists(ref property) => document.has_property(entity_id, property),
            &EntityMatch::And(ref a, ref b) => {
                a.matches(document, entity_id) && b.matches(document, entity_id)
            },
            &EntityMatch::Or(ref a, ref b) => {
                a.matches(document, entity_id) || b.matches(document, entity_id)
            }
        }
    }
    pub fn property_of_interest(&self, property_key: &str) -> bool {
        match self {
            &EntityMatch::Any => false,
            &EntityMatch::TypeName(_) => false,
            &EntityMatch::Name(_) => property_key == "name",
            &EntityMatch::PropertyValueEquals { ref property, .. } => property == property_key,
            &EntityMatch::PropertyValueNotEquals { ref property, .. } => property == property_key,
            &EntityMatch::PropertyExists(ref property) => property == property_key,
            &EntityMatch::And(ref a, ref b) => a.property_of_interest(property_key) || b.property_of_interest(property_key),
            &EntityMatch::Or(ref a, ref b) => a.property_of_interest(property_key) || b.property_of_interest(property_key)
        }
    }
}

impl ToString for EntityMatch {
    fn to_string(&self) -> String {
        match self {
            &EntityMatch::Any => "*".to_string(),
            &EntityMatch::TypeName(ref name) => format!("{}", name),
            &EntityMatch::Name(ref name) => format!("[name={}]", name),
            &EntityMatch::PropertyValueEquals { ref property, ref value } => format!("[{}={}]", property, value.to_string()),
            &EntityMatch::PropertyValueNotEquals { ref property, ref value } => format!("[{}!={}]", property, value.to_string()),
            &EntityMatch::PropertyExists(ref property) => format!("[{}]", property),
            &EntityMatch::And(ref a, ref b) => format!("[{} && {}]", a.to_string(), b.to_string()),
            &EntityMatch::Or(ref a, ref b) => format!("[{} || {}]", a.to_string(), b.to_string())
        }
    }
}
