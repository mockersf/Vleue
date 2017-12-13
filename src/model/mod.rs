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
        impl From<String> for $name {
            fn from(v: String) -> Self {
                $name(v)
            }
        }
        impl $name {
            #[allow(dead_code)]
            fn new() -> $name {
                $name(format!("{}", uuid::Uuid::new_v4().hyphenated()))
            }
        }
    );
}

pub mod app;
pub mod api;
mod domain;

pub use self::app::*;
pub use self::domain::*;
