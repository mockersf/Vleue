use std::env;
use std::collections::HashMap;

use crowbar;
use http;
use mime;
use serde_json;
use uuid;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use failure::{Error, Fail};

use rusoto_core::{DefaultCredentialsProvider, Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, QueryInput, QueryOutput, PutItemInput,
                      AttributeValue};
use rusoto_core::default_tls_client;

use model;

pub struct SerializableError(pub Error);
impl Serialize for SerializableError {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 1)?;
        state.serialize_field("error", &format!("{}", self.0))?;
        state.end()
    }
}
impl<F: Fail> From<F> for SerializableError {
    fn from(failure: F) -> SerializableError {
        SerializableError(failure.into())
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Missing Body")]
struct MissingBody();
#[derive(Debug, Fail)]
#[fail(display = "Missing Field: '{}'", _0)]
struct MissingField(&'static str);
#[derive(Debug, Fail)]
#[fail(display = "Parsing Error: {}", serde_error)]
struct ParsingError {
    #[cause]
    serde_error: ::serde_json::Error,
}
#[derive(Debug, Fail)]
#[fail(display = "Invalid UUID for {}: '{}'", field, uuid)]
struct InvalidUUIDError {
    uuid: String,
    field: &'static str,
    #[cause]
    uuid_error: uuid::ParseError,
}


pub fn list(
    event: &crowbar::Value,
    _context: &crowbar::LambdaContext,
) -> crowbar::LambdaResult<crowbar::ApiGatewayResponse<model::api::ItemList>> {
    let table = env::var("table").unwrap();
    let provider = DefaultCredentialsProvider::new().unwrap();
    let client = DynamoDbClient::new(default_tls_client().unwrap(), provider, Region::UsEast1);
    let mut attributes = HashMap::new();
    attributes.insert(
        ":uid".to_string(),
        AttributeValue {
            s: Some(
                event["requestContext"]["authorizer"]["user_id"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            ),
            ..Default::default()
        },
    );
    let query_input = QueryInput {
        table_name: table,
        expression_attribute_values: Some(attributes),
        key_condition_expression: Some("uid = :uid".to_string()),
        ..Default::default()
    };
    let query_output = client.query(&query_input);
    let a: QueryOutput = query_output.unwrap();
    let r = a.items
        .unwrap()
        .iter()
        .map(|item| {
            let description = item.get("description").and_then(|item| item.s.clone());
            let title = item.get("title").and_then(|item| item.s.clone());
            let id = item.get("id").and_then(|item| item.s.clone());
            let project_id = item.get("project_id").and_then(|item| item.s.clone());
            model::basic_item::BasicItem {
                description: description.unwrap_or_else(|| "".to_string()),
                flagged: false,
                id: id.unwrap().into(),
                project_id: project_id.unwrap().into(),
                status: model::State { name: "".to_string() },
                title: title.unwrap(),
            }
        })
        .collect();

    let todos = model::api::ItemList { items: r };

    Ok(crowbar::ApiGatewayResponse {
        status_code: http::StatusCode::OK,
        body: Some((Ok(todos), mime::APPLICATION_JSON)),
        ..Default::default()
    })
}

#[derive(Deserialize)]
struct ItemInput {
    title: Option<String>,
    description: Option<String>,
    project_id: Option<String>,
}
impl ItemInput {
    fn new_item(self) -> Result<model::basic_item::BasicItem, SerializableError> {
        let id = model::ItemId(format!("{}", uuid::Uuid::new_v4().hyphenated()));
        let title = self.title.clone().ok_or_else(
            || MissingField("title").into(),
        );
        let description = self.description.clone().unwrap_or_else(|| "".to_string());
        let input_project_id = uuid::Uuid::parse_str(
            &self.project_id.clone().unwrap_or_else(|| "".to_string()),
        ).map_err(|err| {
            InvalidUUIDError {
                uuid: self.project_id.clone().unwrap_or_else(|| "".to_string()),
                field: "project_id",
                uuid_error: err,
            }
        })?;
        let project_id = model::ProjectId(format!("{}", input_project_id));
        title.map(|title| {
            model::basic_item::BasicItem {
                description: description,
                flagged: false,
                id: id,
                project_id: project_id,
                status: model::State { name: "".to_string() },
                title: title,
            }
        })
    }
}

pub fn add(
    event: &crowbar::Value,
    _context: &crowbar::LambdaContext,
) -> crowbar::LambdaResult<
    crowbar::ApiGatewayResponse<
        model::basic_item::BasicItem,
        SerializableError,
    >,
> {
    let data_result: Result<ItemInput, SerializableError> = event["body"]
        .as_str()
        .ok_or_else(|| MissingBody().into())
        .and_then(|valid_body| {
            serde_json::from_slice::<ItemInput>(valid_body.as_bytes())
                .map_err(|err| ParsingError { serde_error: err }.into())
        });
    match data_result.and_then(|item| item.new_item()) {
        Ok(item) => {
            let table = env::var("table").unwrap();
            let mut key = HashMap::new();
            key.insert(
                "uid".to_string(),
                AttributeValue {
                    s: Some(
                        event["requestContext"]["authorizer"]["user_id"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                    ),
                    ..Default::default()
                },
            );
            key.insert(
                "id".to_string(),
                AttributeValue {
                    s: Some(item.id.to_string()),
                    ..Default::default()
                },
            );
            key.insert(
                "title".to_string(),
                AttributeValue {
                    s: Some(item.title.to_owned()),
                    ..Default::default()
                },
            );
            key.insert(
                "project_id".to_string(),
                AttributeValue {
                    s: Some(item.project_id.to_string()),
                    ..Default::default()
                },
            );
            if item.description != "" {
                key.insert(
                    "description".to_string(),
                    AttributeValue {
                        s: Some(item.description.to_owned()),
                        ..Default::default()
                    },
                );
            }
            let put_item = PutItemInput {
                item: key,
                table_name: table,
                ..Default::default()
            };
            let provider = DefaultCredentialsProvider::new().unwrap();
            let client =
                DynamoDbClient::new(default_tls_client().unwrap(), provider, Region::UsEast1);
            client.put_item(&put_item).unwrap();
            Ok(crowbar::ApiGatewayResponse {
                status_code: http::StatusCode::OK,
                body: Some((Ok(item), mime::APPLICATION_JSON)),
                ..Default::default()
            })
        }
        Err(error) => {
            Ok(crowbar::ApiGatewayResponse {
                status_code: http::StatusCode::BAD_REQUEST,
                body: Some((Err(error), mime::APPLICATION_JSON)),
                ..Default::default()
            })
        }

    }
}
