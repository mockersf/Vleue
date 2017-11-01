use crowbar;
use serde_json;
use http;
use mime;

use model;

pub fn list_f(event: crowbar::Value, _context: crowbar::LambdaContext) -> crowbar::LambdaResult<crowbar::ApiGatewayResponse> {
    println!("{:?}", event);
    let todos = model::TodoList {
        todos: vec![]
    };

    Ok(crowbar::ApiGatewayResponse {
        status_code: http::StatusCode::OK,
        body: Some((serde_json::to_string(&todos).unwrap(), mime::APPLICATION_JSON)),
        ..Default::default()
    })
}
