

#[macro_export]
macro_rules! pon_doc_expand_map {
    ($fields:expr, { }) => ();
    ($fields:expr, { $name:ident : $inner:tt, $($rest:tt)* }) => (
        $fields.push(PonDocMapField {
            var_name: stringify!($name).to_string(),
            optional: false,
            default: None,
            value: pon_doc_expand!($inner)
        });
        pon_doc_expand_map!($fields, { $($rest)* })
    );
    ($fields:expr, { $name:ident : $inner:tt optional, $($rest:tt)* }) => (
        $fields.push(PonDocMapField {
            var_name: stringify!($name).to_string(),
            optional: true,
            default: None,
            value: pon_doc_expand!($inner)
        });
        pon_doc_expand_map!($fields, { $($rest)* })
    );
    ($fields:expr, { $name:ident : $inner:tt | $default:expr, $($rest:tt)* }) => (
        $fields.push(PonDocMapField {
            var_name: stringify!($name).to_string(),
            optional: false,
            default: Some(stringify!($default).to_string()),
            value: pon_doc_expand!($inner)
        });
        pon_doc_expand_map!($fields, { $($rest)* })
    );
}

#[macro_export]
macro_rules! pon_doc_expand {
    () => ($crate::pon_doc::PonDocMatcher::Nil);
    (( enum { $($id:expr => $val:expr,)+ } )) => ({
        $crate::pon_doc::PonDocMatcher::Enum(vec![$(
            PonDocEnumOption { name: $id.to_string() },
        )+])
    });
    ({ $typ:ty }) => ({
        $crate::pon_doc::PonDocMatcher::Object { typ: stringify!($typ).to_string() }
    });
    ({ $($rest:tt)* }) => ({
        let mut fields = Vec::new();
        pon_doc_expand_map!(fields, { $($rest)* });
        $crate::pon_doc::PonDocMatcher::Map(fields)
    });
    ([ $typ:ty ]) => ({
        $crate::pon_doc::PonDocMatcher::Array { typ: stringify!($typ).to_string() }
    });
    (( $typ:ty )) => (
        $crate::pon_doc::PonDocMatcher::Value { typ: stringify!($typ).to_string() }
    );
    ($name:ident : $t:tt) => (
        $crate::pon_doc::PonDocMatcher::Capture { var_name: stringify!($name).to_string(), value: Box::new(pon_doc_expand!($t)) }
    );
}


#[derive(Clone, Debug, PartialEq)]
pub struct PonDocMapField {
    pub var_name: String,
    pub optional: bool,
    pub default: Option<String>,
    pub value: PonDocMatcher
}

#[derive(Clone, Debug, PartialEq)]
pub struct PonDocEnumOption {
    pub name: String
}

#[derive(Clone, Debug, PartialEq)]
pub enum PonDocMatcher {
    Nil,
    Value {
        typ: String
    },
    Array {
        typ: String
    },
    Object {
        typ: String
    },
    Map(Vec<PonDocMapField>),
    Enum(Vec<PonDocEnumOption>),
    Capture {
        var_name: String,
        value: Box<PonDocMatcher>
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PonDocFunction {
    pub name: String,
    pub target_type_name: String,
    pub arg: PonDocMatcher
}
