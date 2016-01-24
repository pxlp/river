
use document::*;
use pon::*;

#[derive(PartialEq, Eq, Debug, Clone, Hash, PartialOrd, Ord)]
pub enum EntityMatch {
    Any,
    Name(String),
    TypeName(String),
    PropertyValue { property: String, value: Box<Pon> },
    PropertyExists(String),
}

impl EntityMatch {
    pub fn property_value(property: String, value: Pon) -> EntityMatch {
        if property == "name" {
            EntityMatch::Name(match value {
                Pon::String(string) => string.to_string(),
                _ => panic!("Name match must always be a string")
            })
        } else {
            EntityMatch::PropertyValue { property: property, value: Box::new(value) }
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
            &EntityMatch::PropertyValue { ref property, box ref value} => match document.get_property_expression(entity_id, property) {
                Ok(val) => val == value,
                Err(_) => false
            },
            &EntityMatch::PropertyExists(ref property) => match document.has_property(entity_id, property) {
                Ok(val) => val,
                Err(_) => false
            }
        }
    }
    pub fn property_of_interest(&self, property_key: &str) -> bool {
        match self {
            &EntityMatch::Any => false,
            &EntityMatch::TypeName(_) => false,
            &EntityMatch::Name(_) => property_key == "name",
            &EntityMatch::PropertyValue { ref property, .. } => property == property_key,
            &EntityMatch::PropertyExists(ref property) => property == property_key
        }
    }
}

impl ToString for EntityMatch {
    fn to_string(&self) -> String {
        match self {
            &EntityMatch::Any => "*".to_string(),
            &EntityMatch::TypeName(ref name) => format!("{}", name),
            &EntityMatch::Name(ref name) => format!("[name={}]", name),
            &EntityMatch::PropertyValue { ref property, ref value } => format!("[{}={}]", property, value.to_string()),
            &EntityMatch::PropertyExists(ref property) => format!("[{}]", property),
        }
    }
}
