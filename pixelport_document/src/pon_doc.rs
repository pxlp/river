use pad::{PadStr, Alignment};

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

impl PonDocMapField {
    pub fn generate_usage(&self, indentation: usize) -> String {
        format!("{}{}: {}{}{}{}", "".pad_to_width(indentation), self.var_name, self.value.generate_usage(indentation),
            if self.optional || self.default.is_some() {
                " //".to_string()
            } else {
                "".to_string()
            },
            if self.optional { " Optional." } else { "" },
            if let &Some(ref default) = &self.default { format!(" Defaults to {}.", default) } else { "".to_string() })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PonDocEnumOption {
    pub name: String
}

impl PonDocEnumOption {
    pub fn generate_usage(&self) -> String {
        format!(" '{}' ", self.name)
    }
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

impl PonDocMatcher {
    pub fn generate_usage(&self, indentation: usize) -> String {
        match self {
            &PonDocMatcher::Nil => "()".to_string(),
            &PonDocMatcher::Value { ref typ } => format!("<{}>", typ),
            &PonDocMatcher::Array { ref typ } => format!("[ <{}>, ... ]", typ),
            &PonDocMatcher::Object { ref typ } => format!(r#"{{ <key 1>: <{}>, ... }}"#, typ),
            &PonDocMatcher::Map(ref fields) => {
                let fields: Vec<String> = fields.iter().map(|field| field.generate_usage(indentation + 2)).collect();
                format!("{{\n{}\n}}", fields.join(",\n"))
            },
            &PonDocMatcher::Enum(ref options) => {
                let options: Vec<String> = options.iter().map(|option| option.generate_usage()).collect();
                format!("<{}>", options.join("/"))
            },
            &PonDocMatcher::Capture { ref var_name, ref value } => format!("{}", value.generate_usage(indentation))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PonDocFunction {
    pub module: String,
    pub name: String,
    pub target_type_name: String,
    pub arg: PonDocMatcher,
    pub doc: String
}

impl PonDocFunction {
    pub fn generate_md(&self) -> String {
        format!(r#"### {name}
```pon
{name} {arg_usage}

// Returns: {returns}
```

"#, name=self.name, arg_usage=self.arg.generate_usage(0), returns=self.target_type_name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PonDocModule {
    pub name: String,
    pub doc: String,
    pub functions: Vec<PonDocFunction>
}

impl PonDocModule {
    pub fn generate_md(&self) -> String {
        let funs: Vec<String> = self.functions.iter().map(|f| f.generate_md()).collect();
        format!("## {}\n\n{}\n\n{}", self.name, self.doc, funs.join("\n"))
    }
}
