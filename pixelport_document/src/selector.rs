
use document::*;
use pon::*;
use entity_match::*;

// / == next level
// : == search descendants for
// |parent| == parent
// * == match anything
// [] == match what's inside of brackets
// , == or

// @this.property_key
// this|parent|
// this:[alpha=true]/
// this:[alpha=true]/[mesh]
// this:[alpha=true][visible=true]/[mesh]
// this/[alpha=true],[visible=true]/[mesh]
// this/([alpha=true],[visible=true])[dark=false]/[mesh]
// this:*
// this/*
// root
// ()

#[derive(PartialEq, Eq, Debug, Clone, Hash, PartialOrd, Ord)]
pub enum SelectorRoot {
    This,
    Root,
    Id(EntityId),
}
impl ToString for SelectorRoot {
    fn to_string(&self) -> String {
        match self {
            &SelectorRoot::This => "this".to_string(),
            &SelectorRoot::Root => "root".to_string(),
            &SelectorRoot::Id(ref id) => format!("#{}", id),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectorPath {
    Parent,
    Children(EntityMatch),
    Search(EntityMatch),
    SearchInverse(EntityMatch),
    PrevSibling,
    NextSibling,
}
impl SelectorPath {
    fn matches(&self, document: &Document, entity_id: EntityId) -> bool {
        match self {
            &SelectorPath::Parent => unimplemented!(),
            &SelectorPath::Children(ref entity_match) => {
                entity_match.matches(document, entity_id)
            },
            &SelectorPath::Search(ref entity_match) => {
                entity_match.matches(document, entity_id)
            },
            &SelectorPath::SearchInverse(ref entity_match) => {
                !entity_match.matches(document, entity_id)
            },
            &SelectorPath::PrevSibling => unimplemented!(),
            &SelectorPath::NextSibling => unimplemented!(),
        }
    }
    pub fn property_of_interest(&self, property_key: &str) -> bool {
        match self {
            &SelectorPath::Parent => false,
            &SelectorPath::Children(ref entity_match) => {
                entity_match.property_of_interest(property_key)
            },
            &SelectorPath::Search(ref entity_match) => {
                entity_match.property_of_interest(property_key)
            },
            &SelectorPath::SearchInverse(ref entity_match) => {
                entity_match.property_of_interest(property_key)
            },
            &SelectorPath::PrevSibling => false,
            &SelectorPath::NextSibling => false,
        }
    }
    pub fn is_parent(&self) -> bool {
        if let &SelectorPath::Parent = self {
            true
        } else {
            false
        }
    }
    pub fn is_search(&self) -> bool {
        if let &SelectorPath::Search(_) = self {
            true
        } else {
            false
        }
    }
    pub fn is_search_inverse(&self) -> bool {
        if let &SelectorPath::SearchInverse(_) = self {
            true
        } else {
            false
        }
    }
}
impl ToString for SelectorPath {
    fn to_string(&self) -> String {
        match self {
            &SelectorPath::Parent => "|parent|".to_string(),
            &SelectorPath::Children(ref entity_match) => format!("/{}", entity_match.to_string()),
            &SelectorPath::Search(ref entity_match) => format!(":{}", entity_match.to_string()),
            &SelectorPath::SearchInverse(ref entity_match) => format!(":!{}", entity_match.to_string()),
            &SelectorPath::PrevSibling => "|prev-sibling|".to_string(),
            &SelectorPath::NextSibling => "|next-sibling|".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub root: SelectorRoot,
    pub path: Vec<SelectorPath>,
}
//impl_pno!(Selector);
impl ToPon for Selector {
    fn to_pon(&self) -> Pon {
        Pon::Selector(self.clone())
    }
}

enum PathMatchResult {
    Unresolved(usize),
    Resolved(bool)
}

impl Selector {
    pub fn from_string(string: &str) -> Result<Selector, PonParseError> {
        selector_from_string(string)
    }
    pub fn this() -> Selector {
        Selector { root: SelectorRoot::This, path: vec![] }
    }
    pub fn root() -> Selector {
        Selector { root: SelectorRoot::Root, path: vec![] }
    }
    pub fn id(id: EntityId) -> Selector {
        Selector { root: SelectorRoot::Id(id), path: vec![] }
    }
    pub fn this_parent() -> Selector {
        Selector { root: SelectorRoot::This, path: vec![SelectorPath::Parent] }
    }
    pub fn root_search(search: String) -> Selector {
        Selector { root: SelectorRoot::Root, path: vec![SelectorPath::Search(EntityMatch::Name(search))] }
    }
    pub fn root_any() -> Selector {
        Selector { root: SelectorRoot::Root, path: vec![SelectorPath::Search(EntityMatch::Any)] }
    }
    pub fn this_any() -> Selector {
        Selector { root: SelectorRoot::This, path: vec![SelectorPath::Search(EntityMatch::Any)] }
    }
    fn path_match(&self, document: &Document, ent: EntityId, select_root: EntityId, start_path_i: usize, path_stop: usize) -> PathMatchResult {
        let mut path_i = start_path_i;
        let a = match path_i > path_stop + 1 {
            true => Some(&self.path[path_i - 2]),
            false => None
        };
        let b = match path_i > path_stop + 0 {
            true => Some(&self.path[path_i - 1]),
            false => None
        };
        let c = match path_i < self.path.len() {
            true => Some(&self.path[path_i]),
            false => None
        };
        let searchtarget = {
            if let Some(b_) = b {
                if let &SelectorPath::SearchInverse(_) = b_ {
                    a
                } else {
                    b
                }
            } else {
                None
            }
        };
        if a.is_none() && b.is_some() && b.unwrap().is_search_inverse() {
            if !b.unwrap().matches(document, ent) {
                return PathMatchResult::Resolved(false);
            }
            if ent == select_root {
                return PathMatchResult::Resolved(true);
            }
        } else if b.is_none() {
            if let Some(c) = c {
                match c {
                    &SelectorPath::Children(_) => return PathMatchResult::Resolved(ent == select_root),
                    &SelectorPath::Search(_) => {
                        if ent == select_root {
                            return PathMatchResult::Resolved(true);
                        }
                    },
                    &SelectorPath::SearchInverse(ref entity_match) => {
                        if entity_match.matches(document, ent) {
                            return PathMatchResult::Resolved(false);
                        }
                        if ent == select_root {
                            return PathMatchResult::Resolved(true);
                        }
                    },
                    _ => unimplemented!()
                }
            } else {
                return PathMatchResult::Resolved(ent == select_root);
            }
        } else if a.is_some() && b.is_some() && b.unwrap().is_search_inverse() {
            if b.unwrap().matches(document, ent) {
                return PathMatchResult::Resolved(false);
            }
            if a.unwrap().matches(document, ent) {
                path_i -= 2;
            }
        } else {
            if b.unwrap().matches(document, ent) {
                path_i -= 1;
            } else if !(c.is_some() && c.unwrap().is_search()) {
                return PathMatchResult::Resolved(false);
            }
        }

        if path_i != start_path_i && searchtarget.is_some() && searchtarget.unwrap().is_search() {
            self.path_match(document, ent, select_root, path_i, path_stop)
        } else {
            PathMatchResult::Unresolved(path_i)
        }
    }
    pub fn matches(&self, document: &Document, this_entity_id: EntityId, matching_entity_id: EntityId) -> bool {
        let document_root = match document.get_root() {
            Some(root) => root,
            None => panic!("Uninitialized document")
        };
        let mut select_root = match &self.root {
            &SelectorRoot::This => this_entity_id,
            &SelectorRoot::Root => document_root,
            &SelectorRoot::Id(ref id) => *id
        };
        let mut path_stop = 0;
        loop {
            if path_stop < self.path.len() && self.path[path_stop].is_parent() {
                if let Ok(Some(new_root)) = document.get_parent(select_root) {
                    path_stop += 1;
                    select_root = new_root;
                } else {
                    return false;
                }
            } else {
                break;
            }
        }
        let mut ent = matching_entity_id;
        let mut path_i = self.path.len();

        loop {
            path_i = match self.path_match(document, ent, select_root, path_i, path_stop) {
                PathMatchResult::Unresolved(i) => i,
                PathMatchResult::Resolved(res) => return res
            };

            ent = match document.get_parent(ent) {
                Ok(Some(id)) => id,
                _ => return false
            };
        }
    }
    pub fn find_first(&self, document: &Document, this_entity_id: EntityId) -> Result<EntityId, DocError> {
        let mut ent = match &self.root {
            &SelectorRoot::This => this_entity_id,
            &SelectorRoot::Root => match document.get_root() {
                Some(root) => root,
                None => panic!("Uninitialized document")
            },
            &SelectorRoot::Id(ref id) => *id
        };
        for p in &self.path {
            match p {
                &SelectorPath::Parent => {
                    ent = match try!(document.get_parent(ent)) {
                        Some(id) => id,
                        None => panic!("Trying to take parent of root")
                    };
                },
                &SelectorPath::Children(ref entity_match) => {
                    let mut c = None;
                    for child_id in try!(document.get_children(ent)) {
                        if entity_match.matches(document, *child_id) {
                            c = Some(*child_id);
                            break;
                        }
                    }
                    ent = match c {
                        Some(id) => id,
                        None => return Err(DocError::NoSuchEntity(ent))
                    };
                },
                &SelectorPath::Search(ref entity_match) => {
                    ent = try!(self.find_first_descendant(document, ent, entity_match));
                },
                &SelectorPath::SearchInverse(ref entity_match) => {
                    ent = try!(self.find_first_descendant_inverse(document, ent, entity_match));
                },
                &SelectorPath::PrevSibling => {
                    ent = try!(document.get_prev_sibling(ent));
                },
                &SelectorPath::NextSibling => {
                    ent = try!(document.get_next_sibling(ent));
                }
            }
        }
        Ok(ent)
    }
    fn find_first_descendant(&self, document: &Document, entity_id: EntityId, entity_match: &EntityMatch) -> Result<EntityId, DocError> {
        for child_id in try!(document.get_children(entity_id)) {
            if entity_match.matches(document, *child_id) {
                return Ok(*child_id);
            } else if let Ok(match_id) = self.find_first_descendant(document, *child_id, entity_match) {
                return Ok(match_id);
            }
        }
        return Err(DocError::NoSuchEntity(entity_id));
    }
    fn find_first_descendant_inverse(&self, document: &Document, entity_id: EntityId, entity_match: &EntityMatch) -> Result<EntityId, DocError> {
        for child_id in try!(document.get_children(entity_id)) {
            if !entity_match.matches(document, *child_id) {
                return Ok(*child_id);
            }
        }
        return Err(DocError::NoSuchEntity(entity_id));
    }
    pub fn property_of_interest(&self, property_key: &str) -> bool {
        for p in &self.path {
            if p.property_of_interest(property_key) {
                return true;
            }
        }
        return false;
    }
}

impl ToString for Selector {
    fn to_string(&self) -> String {
        let mut string = self.root.to_string();
        for p in &self.path {
            string = string + &p.to_string();
        }
        string
    }
}
