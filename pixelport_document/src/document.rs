use xml;
use pon::*;
use pon_runtime::*;

use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::collections::hash_map::Entry;
use std::path::Path;
use std::io::Write;
use std::any::Any;
use std::marker::Reflect;
use std::cell::RefCell;
use std::borrow::Cow;
use std::fmt;

use xml::reader::EventReader;
use invalidated_properties_cache::*;
use std::mem;

#[derive(PartialEq, Debug, Clone)]
pub enum DocError {
    PonRuntimeErr(PonRuntimeErr),
    NoSuchProperty(String),
    NoSuchEntity(EntityId),
    CantFindEntityByName(String),
    InvalidParent
}
impl ToString for DocError {
    fn to_string(&self) -> String {
        match self {
            &DocError::PonRuntimeErr(ref err) => format!("Pon runtime error: {}", err.to_string()),
            _ => format!("{:?}", self)
        }
    }
}

impl From<PonRuntimeErr> for DocError {
    fn from(err: PonRuntimeErr) -> DocError {
        DocError::PonRuntimeErr(err)
    }
}

pub type EntityId = u64;

pub type EntityIter<'a> = Keys<'a, EntityId, Entity>;
pub type PropertyIter<'a> = Keys<'a, String, Property>;

#[derive(Debug)]
pub struct Property {
    pub expression: Pon,
    value: RefCell<Option<Box<PonNativeObject>>>,
}

#[derive(Debug)]
pub struct Entity {
    pub id: EntityId,
    pub type_name: String,
    pub properties: HashMap<String, Property>,
    pub name: Option<String>,
    pub children_ids: Vec<EntityId>,
    pub parent_id: Option<EntityId>
}

#[derive(Debug)]
pub struct CycleChanges {
    pub set_properties: Vec<PropRef>,
    pub invalidated_properties: Vec<PropRef>,
    pub entities_added: Vec<EntityId>,
    pub entities_removed: Vec<Entity>,
}
impl CycleChanges {
    pub fn new() -> CycleChanges {
        CycleChanges {
            set_properties: vec![],
            invalidated_properties: vec![],
            entities_added: vec![],
            entities_removed: vec![]
        }
    }
    pub fn changed(&self) -> bool {
        return self.entities_added.len() > 0 || self.entities_removed.len() > 0 ||
            self.set_properties.len() > 0 || self.invalidated_properties.len() > 0;
    }
}

pub struct Document {
    id_counter: EntityId,
    root: Option<EntityId>,
    entities: HashMap<EntityId, Entity>,
    entity_ids_by_name: HashMap<String, EntityId>,
    pub resources: HashMap<String, Box<Any>>,
    pub runtime: PonRuntime,
    invalidated_properties: InvalidatedPropertiesCache,
    this_cycle_changes: CycleChanges
}

impl Document {
    pub fn new() -> Document {
        Document {
            id_counter: 0,
            root: None,
            entities: HashMap::new(),
            entity_ids_by_name: HashMap::new(),
            resources: HashMap::new(),
            runtime: PonRuntime::new(),
            invalidated_properties: InvalidatedPropertiesCache::new(),
            this_cycle_changes: CycleChanges::new(),
        }
    }
    pub fn new_with_root() -> Document {
        let mut doc = Document::new();
        doc.append_entity(None, "Pml", None).unwrap();
        doc
    }
    fn new_id(&mut self) -> EntityId {
        self.id_counter += 1;
        return self.id_counter;
    }
    pub fn append_entity(&mut self, parent_id: Option<EntityId>, type_name: &str, name: Option<String>) -> Result<EntityId, DocError> {
        let id = self.new_id();
        let entity = Entity {
            id: id.clone(),
            type_name: type_name.to_string(),
            properties: HashMap::new(),
            name: name,
            parent_id: parent_id,
            children_ids: vec![]
        };
        if let Some(parent_id) = parent_id {
            let parent = match self.entities.get_mut(&parent_id) {
                Some(parent) => parent,
                None => return Err(DocError::InvalidParent)
            };
            parent.children_ids.push(id);
        } else {
            if self.root.is_some() {
                panic!("Cannot set root twice.");
            }
            self.root = Some(id);
        }
        if let &Some(ref name) = &entity.name {
            self.entity_ids_by_name.insert(name.clone(), entity.id);
        }
        self.entities.insert(entity.id, entity);
        self.this_cycle_changes.entities_added.push(id);
        return Ok(id);
    }
    pub fn get_entity_by_name(&self, name: &str) -> Option<EntityId> {
        match self.entity_ids_by_name.get(&name.to_string()) {
            Some(id) => Some(id.clone()),
            None => None
        }
    }
    pub fn entities_iter(&self) -> EntityIter {
        self.entities.keys()
    }
    pub fn get_root(&self) -> Option<EntityId> {
        self.root.clone()
    }
    pub fn set_property(&mut self, entity_id: EntityId, property_key: &str, mut expression: Pon) -> Result<(), DocError> {
        try!(self.resolve_pon_dependencies(entity_id, &mut expression));
        let prop_ref = PropRef::new(entity_id, property_key);
        let mut dependencies = vec![];
        expression.build_dependencies_array(&mut dependencies);
        {
            let mut ent_mut = match self.entities.get_mut(&entity_id) {
                Some(ent) => ent,
                None => return Err(DocError::NoSuchEntity(entity_id))
            };
            match ent_mut.properties.entry(property_key.to_string()) {
                Entry::Occupied(o) => {
                    let o = o.into_mut();
                    if o.expression == expression {
                        return Ok(()); // Early exit so that we don't clear the cache, invalidate it and add it to set properties
                    }
                    o.expression = expression;
                    *o.value.borrow_mut() = None;
                },
                Entry::Vacant(v) => {
                    v.insert(Property {
                        expression: expression,
                        value: RefCell::new(None)
                    });
                }
            }
        }
        self.invalidated_properties.on_property_set(&prop_ref, dependencies);
        self.this_cycle_changes.set_properties.push(prop_ref);
        Ok(())
    }
    pub fn get_property<T: Clone + Reflect + 'static>(&self, entity_id: EntityId, property_key: &str) -> Result<T, DocError> {
        match try!(self.get_property_raw(entity_id, property_key)).as_any().downcast_ref::<T>() {
            Some(v) => Ok(v.clone()),
            None => {
                let to_type_name = unsafe {
                    ::std::intrinsics::type_name::<T>()
                };
                Err(PonRuntimeErr::ValueOfUnexpectedType {
                    found_value: match self.get_property_expression(entity_id, property_key) {
                        Ok(pon) => pon.to_string(),
                        Err(_) => "No prop found".to_string()
                    },
                    expected_type: to_type_name.to_string()
                }.into())
            }
        }
    }
    pub fn get_property_raw(&self, entity_id: EntityId, property_key: &str) -> Result<Box<PonNativeObject>, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => match entity.properties.get(property_key) {
                Some(property) => {
                    let has_value = { property.value.borrow().is_some() };
                    match has_value {
                        true => match &*property.value.borrow() {
                            &Some(ref value) => Ok(value.clone_to_pno()),
                            _ => unreachable!()
                        },
                        false => {
                            let new_value = match self.runtime.translate_raw(&property.expression, self) {
                                Ok(v) => v,
                                Err(err) => return Err(From::from(err))
                            };
                            let new_value_clone = new_value.clone_to_pno();
                            *property.value.borrow_mut() = Some(new_value);
                            Ok(new_value_clone)
                        }
                    }
                },
                None => Err(DocError::NoSuchProperty(property_key.to_string()))
            },
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn has_property(&self, entity_id: EntityId, name: &str) -> Result<bool, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => match entity.properties.get(name) {
                Some(prop) => Ok(true),
                None => Ok(false)
            },
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn close_cycle(&mut self) -> CycleChanges {
        let mut cycle_changes = mem::replace(&mut self.this_cycle_changes, CycleChanges::new());
        cycle_changes.invalidated_properties = self.invalidated_properties.close_cycle();
        cycle_changes.invalidated_properties.retain(|prop_ref| {
            // Clear out caches and only keep props that are actually around still
            match self.clear_property_cache(prop_ref.entity_id, &prop_ref.property_key) {
                Ok(_) => true,
                Err(_) => false
            }
        });
        return cycle_changes;
    }
    pub fn clear_all_property_caches(&self) {
        for (_, entity) in self.entities.iter() {
            for (_, property) in entity.properties.iter() {
                *property.value.borrow_mut() = None;
            }
        }
    }
    fn clear_property_cache(&self, entity_id: EntityId, property_key: &str) -> Result<(), DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => match entity.properties.get(property_key) {
                Some(property) => {
                    *property.value.borrow_mut() = None;
                    Ok(())
                },
                None => Err(DocError::NoSuchProperty(property_key.to_string()))
            },
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_property_expression(&self, entity_id: EntityId, property_key: &str) -> Result<&Pon, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => match entity.properties.get(property_key) {
                Some(property) => Ok(&property.expression),
                None => Err(DocError::NoSuchProperty(property_key.to_string()))
            },
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_properties(&self, entity_id: EntityId) -> Result<Vec<PropRef>, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => Ok(entity.properties.keys().map(|key| PropRef { entity_id: entity_id.clone(), property_key: key.clone() }).collect()),
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_children(&self, entity_id: EntityId) -> Result<&Vec<EntityId>, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => Ok(&entity.children_ids),
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_entity_type_name(&self, entity_id: EntityId) -> Result<String, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => Ok(entity.type_name.clone()),
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_entity_name(&self, entity_id: EntityId) -> Result<&Option<String>, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => Ok(&entity.name),
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_parent(&self, entity_id: EntityId) -> Result<Option<EntityId>, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => Ok(entity.parent_id),
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_prev_sibling(&self, entity_id: EntityId) -> Result<EntityId, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => {
                match entity.parent_id {
                    Some(parent_id) => {
                        let parent = self.entities.get(&parent_id).unwrap();
                        let e_pos = parent.children_ids.iter().position(|x| *x == entity_id).unwrap();
                        if e_pos > 0 {
                            Ok(parent.children_ids[(e_pos - 1) as usize])
                        } else {
                            Err(DocError::NoSuchEntity(entity_id))
                        }
                    },
                    None => Err(DocError::NoSuchEntity(entity_id))
                }
            },
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn get_next_sibling(&self, entity_id: EntityId) -> Result<EntityId, DocError> {
        match self.entities.get(&entity_id) {
            Some(entity) => {
                match entity.parent_id {
                    Some(parent_id) => {
                        let parent = self.entities.get(&parent_id).unwrap();
                        let e_pos = parent.children_ids.iter().position(|x| *x == entity_id).unwrap();
                        if e_pos < parent.children_ids.len() - 1 {
                            Ok(parent.children_ids[e_pos + 1])
                        } else {
                            Err(DocError::NoSuchEntity(entity_id))
                        }
                    },
                    None => Err(DocError::NoSuchEntity(entity_id))
                }
            },
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn remove_entity(&mut self, entity_id: EntityId) -> Result<(), DocError> {
        match self.entities.remove(&entity_id) {
            Some(entity) => {
                if let &Some(ref parent_id) = &entity.parent_id {
                    match self.entities.get_mut(parent_id) {
                        Some(parent) => parent.children_ids.retain(|id| *id != entity_id),
                        None => {} // We're in a child of a removed entity and the removed entity is already removed
                    }
                }
                for e in &entity.children_ids {
                    try!(self.remove_entity(*e))
                }
                self.this_cycle_changes.entities_removed.push(entity);
                Ok(())
            },
            None => Err(DocError::NoSuchEntity(entity_id))
        }
    }
    pub fn clear_children(&mut self, entity_id: EntityId) -> Result<(), DocError> {
        let children = match self.entities.get(&entity_id) {
            Some(entity) => { entity.children_ids.clone() }
            None => return Err(DocError::NoSuchEntity(entity_id))
        };
        for e in children {
            try!(self.remove_entity(e));
        }
        Ok(())
    }
    pub fn entity_to_string(&self, entity_id: EntityId) -> Result<String, DocError> {
        let mut buff = vec![];
        {
            let mut writer = xml::writer::EmitterConfig::new()
                .perform_indent(false)
                .write_document_declaration(false)
                .line_separator(" ")
                .create_writer(&mut buff);
            self.entity_to_xml(entity_id, &mut writer);
        }
        Ok(String::from_utf8(buff).unwrap())
    }

    pub fn from_file(path: &Path) -> Result<Document, DocError> {
        let mut doc = Document::new();
        let mut warnings = vec![];
        try!(doc.append_from_event_reader(&mut vec![], event_reader_from_file(path).into_iter(), &mut warnings));
        if warnings.len() > 0 {
            warn!("{} warnings while parsning document:", warnings.len());
            for w in warnings {
                warn!("{}", w);
            }
        }
        Ok(doc)
    }
    pub fn from_string(string: &str) -> Result<Document, DocError> {
        let mut doc = Document::new();
        let parser = EventReader::from_str(string);
        let mut warnings = vec![];
        try!(doc.append_from_event_reader(&mut vec![], parser.into_iter(), &mut warnings));
        if warnings.len() > 0 {
            warn!("{} warnings while parsning document:", warnings.len());
            for w in warnings {
                warn!("{}", w);
            }
        }
        Ok(doc)
    }

    fn resolve_pon_dependencies(&mut self, entity_id: EntityId, node: &mut Pon) -> Result<(), DocError> {
        match node {
            &mut Pon::PonCall(box PonCall { ref mut arg, .. }) =>
                try!(self.resolve_pon_dependencies(entity_id, arg)),
            &mut Pon::DependencyReference(ref named_prop_ref, ref mut resolved) => {
                let prop_ref = try!(named_prop_ref.resolve(&self, entity_id));
                *resolved = Some(prop_ref);
            },
            &mut Pon::Object(ref mut hm) => {
                for (_, v) in hm.iter_mut() {
                    try!(self.resolve_pon_dependencies(entity_id, v))
                }
            },
            &mut Pon::Array(ref mut arr) => {
                for v in arr.iter_mut() {
                    try!(self.resolve_pon_dependencies(entity_id, v))
                }
            },
            _ => {}
        };
        Ok(())
    }

    fn append_from_event_reader<T: Iterator<Item=xml::reader::Result<xml::reader::XmlEvent>>>(&mut self, mut entity_stack: &mut Vec<EntityId>, mut events: T, warnings: &mut Vec<String>) -> Result<(), DocError> {
        while let Some(e) = events.next() {
            match e {
                Ok(xml::reader::XmlEvent::StartElement { name: type_name, attributes, .. }) => {
                    let entity_name = match attributes.iter().find(|x| x.name.local_name == "name") {
                        Some(attr) => Some(attr.value.to_string()),
                        None => None
                    };
                    let parent = match entity_stack.last() {
                        Some(parent) => Some(*parent),
                        None => None
                    };
                    let entity_id = match self.append_entity(parent, &type_name.local_name, entity_name) {
                        Ok(id) => id,
                        Err(err) => {
                            warnings.push(format!("Failed to append entity {:?}: {:?}", type_name.local_name, err));
                            continue;
                        }
                    };

                    for attribute in attributes {
                        if attribute.name.local_name == "name" { continue; }
                        match Pon::from_string(&attribute.value) {
                            Ok(node) => match self.set_property(entity_id, &attribute.name.local_name, node) {
                                Ok(_) => {},
                                Err(err) => warnings.push(format!("Failed to set property {} for entity {:?}: {:?}", attribute.name.local_name, type_name.local_name, err))
                            },
                            Err(err) => warnings.push(format!("Error parsing property {} of entity {:?}: {} with error: {:?}", attribute.name.local_name, type_name.local_name, attribute.value, err))
                        };
                    }
                    entity_stack.push(entity_id);
                }
                Ok(xml::reader::XmlEvent::EndElement { .. }) => {
                    entity_stack.pop();
                }
                Err(e) => {
                    warnings.push(format!("Xml parsing error: {}", e));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn entity_to_xml<T: Write>(&self, entity_id: EntityId, writer: &mut xml::writer::EventWriter<T>) {
        let entity = self.entities.get(&entity_id).unwrap();
        let type_name = xml::name::Name::local(&entity.type_name);
        let mut attrs: Vec<xml::attribute::OwnedAttribute> = entity.properties.iter().filter_map(|(name, prop)| {
            Some(xml::attribute::OwnedAttribute {
                name: xml::name::OwnedName::local(name.to_string()),
                value: prop.expression.to_string()
            })
        }).collect();
        if let &Some(ref name) = &entity.name {
            attrs.push(xml::attribute::OwnedAttribute {
                name: xml::name::OwnedName::local("name"),
                value: name.to_string()
            });
        }
        attrs.sort_by(|a, b| a.name.local_name.cmp(&b.name.local_name) );
        writer.write(xml::writer::events::XmlEvent::StartElement {
            name: type_name.clone(),
            attributes: attrs.iter().map(|x| x.borrow()).collect(),
            namespace: Cow::Owned(xml::namespace::Namespace::empty())
        }).unwrap();
        for e in &entity.children_ids {
            self.entity_to_xml(*e, writer);
        }
        writer.write(xml::writer::events::XmlEvent::EndElement {
            name: Some(type_name.clone())
        }).unwrap();
    }
    fn to_xml(&self) -> String {
        let mut buff = vec![];
        {
            let mut writer = xml::writer::EmitterConfig::new()
                .perform_indent(true)
                .create_writer(&mut buff);
            writer.write(xml::writer::events::XmlEvent::StartDocument {
                version: xml::common::XmlVersion::Version11,
                encoding: None,
                standalone: None
            }).unwrap();
            if self.root.is_some() {
                self.entity_to_xml(self.root.unwrap(), &mut writer);
            }
        }
        String::from_utf8(buff).unwrap()
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Document {{ root: {:?}, entities: {:?} }}", self.root, self.entities)
    }
}

fn event_reader_from_file(path: &Path) -> EventReader<BufReader<File>> {
    let file = File::open(path).unwrap();
    let file = BufReader::new(file);

    EventReader::new(file)
}

impl ToString for Document {
    fn to_string(&self) -> String {
        self.to_xml()
    }
}
