use std::env;
use std::collections::HashMap;

use crowbar;
use http;
use mime;
use serde_json;
use uuid;

use rusoto_core::{DefaultCredentialsProvider, Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, QueryInput, QueryOutput, PutItemInput, AttributeValue};
use rusoto_core::default_tls_client;

use model;

pub fn list(event: crowbar::Value, _context: crowbar::LambdaContext) -> crowbar::LambdaResult<crowbar::ApiGatewayResponse<model::api::ItemList>> {
    let table = env::var("table").unwrap();
    let provider = DefaultCredentialsProvider::new().unwrap();
    let client = DynamoDbClient::new(default_tls_client().unwrap(), provider, Region::UsEast1);
    let mut attributes = HashMap::new();
    attributes.insert(":uid".to_string(), AttributeValue {
        s: Some(event["requestContext"]["authorizer"]["user_id"].as_str().unwrap().to_string()),
        ..Default::default()
    });
    let query_input = QueryInput {
        table_name: table,
        expression_attribute_values: Some(attributes),
        key_condition_expression: Some("uid = :uid".to_string()),
        ..Default::default()
    };
    let query_output = client.query(&query_input);
    let a: QueryOutput = query_output.unwrap();
    let r = a.items.unwrap().iter().map(|item| {
        let description = item.get("description").and_then(|item| item.s.clone());
        let title = item.get("title").and_then(|item| item.s.clone());
        let id = item.get("id").and_then(|item| item.s.clone());
        model::basic_item::BasicItem {
            description: description.unwrap(),
            flagged: false,
            id: id.unwrap().into(),
            project: model::Project {
                name: "inbox".to_string(),
                workflow: model::Workflow {
                    states: vec![],
                    transitions: vec![],
                },
                costs_info: model::CostInfo {
                    categories: vec![],
                    unit: "hour".to_string(),
                },
            },
            status: model::State {
                name: "".to_string(),
            },
            tags: vec![],
            title: title.unwrap(),
        }
    }).collect();

    let todos = model::api::ItemList {
        items: r
    };

    Ok(crowbar::ApiGatewayResponse {
        status_code: http::StatusCode::OK,
        body: Some((todos, mime::APPLICATION_JSON)),
        ..Default::default()
    })
}

#[derive(Deserialize)]
struct ItemInput {
    title: String,
    description: String,
}

pub fn add(event: crowbar::Value, _context: crowbar::LambdaContext) -> crowbar::LambdaResult<crowbar::ApiGatewayResponse<model::basic_item::BasicItem>> {
    let body = event["body"].as_str();
    let data_result = serde_json::from_slice::<ItemInput>(body.unwrap().as_bytes()).unwrap();
    let id = model::ItemId(format!("{}", uuid::Uuid::new_v4().hyphenated()));
    let item = model::basic_item::BasicItem {
            description: data_result.description,
            flagged: false,
            id: id,
            project: model::Project {
                name: "inbox".to_string(),
                workflow: model::Workflow {
                    states: vec![],
                    transitions: vec![],
                },
                costs_info: model::CostInfo {
                    categories: vec![],
                    unit: "hour".to_string(),
                },
            },
            status: model::State {
                name: "".to_string(),
            },
            tags: vec![],
            title: data_result.title,
    };

    let table = env::var("table").unwrap();
    let provider = DefaultCredentialsProvider::new().unwrap();
    let client = DynamoDbClient::new(default_tls_client().unwrap(), provider, Region::UsEast1);
    let mut key = HashMap::new();
    key.insert("uid".to_string(), AttributeValue {
        s: Some(event["requestContext"]["authorizer"]["user_id"].as_str().unwrap().to_string()),
        ..Default::default()
    });
    key.insert("id".to_string(), AttributeValue {
        s: Some(item.id.to_string()),
        ..Default::default()
    });
    key.insert("title".to_string(), AttributeValue {
        s: Some(item.title.to_owned()),
        ..Default::default()
    });
    key.insert("description".to_string(), AttributeValue {
        s: Some(item.description.to_owned()),
        ..Default::default()
    });
    let put_item = PutItemInput {
        item: key,
        table_name: table,
        ..Default::default()
    };
    client.put_item(&put_item).unwrap();
    Ok(crowbar::ApiGatewayResponse {
        status_code: http::StatusCode::OK,
        body: Some((item, mime::APPLICATION_JSON)),
        ..Default::default()
    })
}
