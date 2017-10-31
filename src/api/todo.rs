use crowbar;
use serde_json;
use http;
use mime;

use model;

pub fn list_f(event: crowbar::Value, _context: crowbar::LambdaContext) -> crowbar::LambdaResult<crowbar::result_helper::ApiGatewayResponse> {
    println!("{:?}", event);
    let todos = model::TodoList {
        todos: vec![]
    };

    crowbar::result_helper::api_gateway_response(http::StatusCode::OK, Some((serde_json::to_string(&todos).unwrap(), mime::APPLICATION_JSON)), None)
}
