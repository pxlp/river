
#[macro_export]
macro_rules! hashmap(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert(::std::convert::From::from($key), $value);
            )+
            m
        }
     };
);
