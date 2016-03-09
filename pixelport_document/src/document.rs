use xml;
use pon::*;
use pon_translater::*;
use bus::*;

use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::path::Path;
use std::io::Write;
use std::any::Any;
use std::fmt;
use std::rc::Rc;

use xml::reader::EventReader;
use std::mem;
use std::borrow::Cow;

#[derive(PartialEq, Debug, Clone)]
pub enum DocError {
    BusError(BusError),
    NoSuchProperty { prop_ref: PropRef },
    NoSuchEntity(EntityId),
    CantFindEntityByName(String),
    InvalidParent
}
impl ToString for DocError {
    fn to_string(&self) -> String {
        match self {
            &DocError::BusError(ref err) => format!("BusError({})", err.to_string()),
            _ => format!("{:?}", self)
        }
    }
}

pub type EntityId = u64;

pub type EntityIter<'a> = Keys<'a, EntityId, Entity>;

#[derive(Debug)]
pub struct Entity {
    pub id: EntityId,
    pub type_name: String,
    pub name: Option<String>,
    pub children_ids: Vec<EntityId>,
    pub parent_id: Option<EntityId>
}

#[derive(Debug)]
pub struct CycleChanges {
    pub invalidations_log: Vec<InvalidatedChange>,
    pub entities_added: Vec<EntityId>,
    pub entities_removed: Vec<Entity>,
}
impl CycleChanges {
    pub fn new() -> CycleChanges {
        CycleChanges {
            invalidations_log: vec![],
            entities_added: vec![],
            entities_removed: vec![]
        }
    }
    pub fn changed(&self) -> bool {
        return self.entities_added.len() > 0 || self.entities_removed.len() > 0 ||
            self.invalidations_log.len() > 0;
    }
}

pub struct EntityIdsReservation {
    pub min: EntityId,
    pub max: EntityId
}

pub struct Document {
    id_counter: EntityId,
    root: Option<EntityId>,
    entities: HashMap<EntityId, Entity>,
    entity_ids_by_name: HashMap<String, EntityId>,
    pub resources: HashMap<String, Box<Any>>,
    pub translater: Rc<PonTranslater>,
    pub bus: Bus,
    property_expressions: HashMap<PropRef, Pon>,
    this_cycle_changes: CycleChanges
}

impl From<BusError> for DocError {
    fn from(err: BusError) -> DocError {
        DocError::BusError(err)
    }
}

impl Document {
    pub fn new(translater: PonTranslater) -> Document {
        Document {
            id_counter: 0,
            root: None,
            entities: HashMap::new(),
            entity_ids_by_name: HashMap::new(),
            resources: HashMap::new(),
            translater: Rc::new(translater),
            bus: Bus::new(),
            property_expressions: HashMap::new(),
            this_cycle_changes: CycleChanges::new(),
        }
    }
    pub fn new_with_root(translater: PonTranslater) -> Document {
        let mut doc = Document::new(translater);
        doc.append_entity(None, None, "Pml", None).unwrap();
        doc
    }
    fn new_id(&mut self) -> EntityId {
        self.id_counter += 1;
        return self.id_counter;
    }
    pub fn append_entity(&mut self, entity_id: Option<EntityId>, parent_id: Option<EntityId>, type_name: &str, name: Option<String>) -> Result<EntityId, DocError> {
        let id = match entity_id {
            Some(id) => id,
            None => self.new_id()
        };
        let entity = Entity {
            id: id,
            type_name: type_name.to_string(),
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
    pub fn set_property(&mut self, entity_id: EntityId, property_key: &str, mut expression: Pon, volatile: bool) -> Result<(), DocError> {
        let prop_ref = PropRef::new(entity_id, property_key);
        try!(self.resolve_pon_dependencies(entity_id, &mut expression));
        let mut dependencies = vec![];
        expression.build_dependencies_array(&mut dependencies);
        let rt = self.translater.clone();
        self.property_expressions.insert(prop_ref.clone(), expression.clone());
        self.bus.set_constructor(&prop_ref.clone(), dependencies, volatile, Box::new(move |bus| {
            match rt.translate_raw(&expression, bus) {
                Ok(v) => Ok(v),
                Err(err) => {
                    warn!("Failed to translate pon to value: {}", err.to_string());
                    Err(BusError::PonTranslateError { err: err, prop_ref: prop_ref.clone() })
                }
            }
        }));
        Ok(())
    }
    pub fn get_property<T: BusValue>(&self, entity_id: EntityId, property_key: &str) -> Result<T, BusError> {
        self.bus.get_typed::<T>(&PropRef::new(entity_id, property_key))
    }
    pub fn get_property_raw(&self, entity_id: EntityId, property_key: &str) -> Result<Box<BusValue>, BusError> {
        self.bus.get(&PropRef::new(entity_id, property_key))
    }
    pub fn get_property_expression(&self, prop_ref: &PropRef) -> Result<&Pon, DocError> {
        match self.property_expressions.get(prop_ref) {
            Some(v) => Ok(v),
            None => Err(DocError::NoSuchProperty { prop_ref: prop_ref.clone() })
        }
    }
    pub fn has_property(&self, entity_id: EntityId, property_key: &str) -> bool {
        self.bus.has(&PropRef::new(entity_id, property_key))
    }
    pub fn close_cycle(&mut self) -> CycleChanges {
        let mut cycle_changes = mem::replace(&mut self.this_cycle_changes, CycleChanges::new());
        self.bus.clear_cache();
        cycle_changes.invalidations_log = mem::replace(&mut self.bus.invalidations_log, Vec::new());
        return cycle_changes;
    }
    pub fn get_properties(&self, entity_id: EntityId) -> Result<Vec<PropRef>, DocError> {
        if !self.entities.contains_key(&entity_id) { return Err(DocError::NoSuchEntity(entity_id)); }
        Ok(self.get_properties_for_entity(entity_id))
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
                self.remove_properties_for_entity(entity_id);
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
    pub fn reserve_entity_ids(&mut self, count: u64) -> EntityIdsReservation {
        self.id_counter += count + 1;
        EntityIdsReservation {
            min: self.id_counter - count,
            max: self.id_counter
        }
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

    pub fn from_file(translater: PonTranslater, path: &Path) -> Result<Document, DocError> {
        let mut doc = Document::new(translater);
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
    pub fn from_string(translater: PonTranslater, string: &str) -> Result<Document, DocError> {
        let mut doc = Document::new(translater);
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
    fn get_properties_for_entity(&self, entity_id: EntityId) -> Vec<PropRef> {
        self.bus.iter().filter_map(|k| {
            if k.entity_id == entity_id {
                Some(k.clone())
            } else {
                None
            }
        }).collect()
    }
    fn remove_properties_for_entity(&mut self, entity_id: EntityId) {
        let props = self.get_properties_for_entity(entity_id);
        for pr in props {
            self.bus.remove(&pr);
            self.property_expressions.remove(&pr);
        }
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
                    let entity_id = match self.append_entity(None, parent, &type_name.local_name, entity_name) {
                        Ok(id) => id,
                        Err(err) => {
                            warnings.push(format!("Failed to append entity {:?}: {:?}", type_name.local_name, err));
                            continue;
                        }
                    };

                    for attribute in attributes {
                        if attribute.name.local_name == "name" { continue; }
                        match Pon::from_string(&attribute.value) {
                            Ok(node) => match self.set_property(entity_id, &attribute.name.local_name, node, false) {
                                Ok(_) => {},
                                Err(err) => warnings.push(format!("Failed to set property {} for entity {:?}: {:?}", attribute.name.local_name, type_name.local_name, err))
                            },
                            Err(PonParseError { line, column, expected, .. }) => {
                                let mut pon_with_line_nrs = "".to_string();
                                let lines: Vec<&str> = attribute.value.split("\n").collect();
                                for i in 0..lines.len() {
                                    pon_with_line_nrs = pon_with_line_nrs + lines[i] + "\n";
                                    if line == i + 1 {
                                        for _ in 1..column {
                                            pon_with_line_nrs = pon_with_line_nrs + " ";
                                        }
                                        pon_with_line_nrs = pon_with_line_nrs + &format!("^ Expected: {:?}\n", expected);
                                    }
                                }
                                warnings.push(format!("Parse error in {}.{}:\n{}",
                                    type_name.local_name, attribute.name.local_name, pon_with_line_nrs))
                            }
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
        let props = self.get_properties_for_entity(entity_id);
        let mut attrs: Vec<xml::attribute::OwnedAttribute> = props.iter().filter_map(|prop_ref| {
            Some(xml::attribute::OwnedAttribute {
                name: xml::name::OwnedName::local(prop_ref.property_key.to_string()),
                value: match self.get_property_expression(prop_ref) {
                    Ok(v) => v.to_string(),
                    Err(_) => "Native Code".to_string()
                }
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
