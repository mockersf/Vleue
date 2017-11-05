macro_rules! typed_id {
    ($name:ident) => (
        #[derive(Serialize, Deserialize, Debug)]
        pub struct $name (pub String);
    );
}

pub mod app;
pub mod api;
mod domain;

pub use self::domain::*;
