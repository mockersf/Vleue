macro_rules! typed_id {
    ($name:ident) => (
        #[derive(Serialize, Deserialize, Debug)]
        pub struct $name (pub String);
        impl ToString for $name {
            fn to_string(&self) -> String {
                let $name(ref string) = *self;
                return string.to_string();
            }
        }
    );
}

pub mod app;
pub mod api;
mod domain;

pub use self::domain::*;
