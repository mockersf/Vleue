use std::env;

use crowbar;
use http;
use mime;
use serde_json;
use uuid;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use failure::{Error, Fail};
use serde_dynamodb;

use rusoto_core::{DefaultCredentialsProvider, Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, QueryInput, PutItemInput};
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

#[derive(Serialize, Debug)]
struct UidFilter {
    #[serde(rename = ":uid")]
    uid: String,
}

pub fn list(
    event: &crowbar::Value,
    _context: &crowbar::LambdaContext,
) -> crowbar::LambdaResult<crowbar::ApiGatewayResponse<model::api::ItemList>> {
    let table = env::var("table").unwrap();
    let provider = DefaultCredentialsProvider::new().unwrap();
    let client = DynamoDbClient::new(default_tls_client().unwrap(), provider, Region::UsEast1);
    let uid_filter = UidFilter {
        uid: event["requestContext"]["authorizer"]["user_id"]
            .as_str()
            .unwrap()
            .to_string(),
    };
    let query_input = QueryInput {
        table_name: table,
        expression_attribute_values: Some(serde_dynamodb::to_hashmap(&uid_filter).unwrap()),
        key_condition_expression: Some("uid = :uid".to_string()),
        ..Default::default()
    };
    let query_output: Vec<model::basic_item::BasicItem> = client
        .query(&query_input)
        .unwrap()
        .items
        .unwrap_or_else(|| vec![])
        .into_iter()
        .map(|item| serde_dynamodb::from_hashmap(item).unwrap())
        .collect();
    let todos = model::api::ItemList { items: query_output };

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
    fn to_new_item(
        &self,
        user_id: model::UserId,
    ) -> Result<model::basic_item::BasicItem, SerializableError> {
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
                uid: user_id,
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
    match data_result.and_then(|item| {
        item.to_new_item(
            event["requestContext"]["authorizer"]["user_id"]
                .as_str()
                .unwrap()
                .to_string()
                .into(),
        )
    }) {
        Ok(item) => {
            let table = env::var("table").unwrap();
            let put_item = PutItemInput {
                item: serde_dynamodb::to_hashmap(&item).unwrap(),
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
