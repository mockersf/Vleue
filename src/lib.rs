#![deny(warnings)]

#[macro_use(lambda)]
extern crate crowbar;
#[macro_use]
extern crate cpython;

#[macro_use]
extern crate failure;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_urlencoded;

extern crate uuid;
extern crate frank_jwt;
extern crate time;
extern crate http;
extern crate mime;
extern crate chrono;

extern crate rusoto_core;
extern crate rusoto_dynamodb;

mod model;
mod api;

pub const JWT_PUB_KEY: &str = "keys/jwtRS256.key.pub";
pub const JWT_SECRET_KEY: &str = "keys/jwtRS256.key";


lambda!(
    "api_auth_get_certificate" => api::auth::get_pub_certificate,
    "api_auth_test_token" => api::auth::test_token,
    "api_auth_check_authorization" => api::auth::check_authorization,
    "api_todo_list" => api::todo::list,
    "api_todo_add" => api::todo::add
);
