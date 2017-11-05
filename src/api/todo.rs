use crowbar;
use http;
use mime;

use model;

pub fn list(event: crowbar::Value, _context: crowbar::LambdaContext) -> crowbar::LambdaResult<crowbar::ApiGatewayResponse<model::api::ItemList>> {
    println!("{:?}", event);
    let todos = model::api::ItemList {
        items: vec![]
    };

    Ok(crowbar::ApiGatewayResponse {
        status_code: http::StatusCode::OK,
        body: Some((todos, mime::APPLICATION_JSON)),
        ..Default::default()
    })
}
