use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

use crowbar::{PyException, Value, LambdaContext, LambdaResult, Policy, ApiGatewayResponse};
use serde_json;
use serde_urlencoded;

use frank_jwt::{Header, Payload, Algorithm, encode, decode};
use uuid;
use time;
use http;
use mime;

use JWT_PUB_KEY;
use JWT_SECRET_KEY;

mod errors {
    error_chain!{
        types {
            Error, ErrorKind, ResultExt, Result;
        }
        foreign_links {
            Fmt(::std::fmt::Error);
            Io(::std::io::Error);
            Serde(::serde::de::value::Error);
        }
    }
}
use self::errors::*;
impl ::std::convert::Into<::crowbar::Error> for Error {
    fn into(self) -> ::crowbar::Error {
        ::crowbar::Error::with_chain(self, ::crowbar::RustError)
    }
}

#[derive(Serialize, Debug, Clone, Default)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u32,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Oauth2ErrorMessage {
    InvalidRequest,
/*    InvalidClient,
    InvalidGrant,
    UnauthorizedClient,
    UnsupportedGrantType,
    InvalidScope,*/
}

#[derive(Serialize, Debug)]
pub struct Oauth2Error {
    error: Oauth2ErrorMessage,
    error_description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TestTokenInput {
    //TODO: use an app token instead of an app_id
    user_id: String,
    app_id: String,
}

pub fn test_token(event: Value, _context: LambdaContext) -> LambdaResult<ApiGatewayResponse> {
    println!("{:?}", event);

    let body = event["body"].as_str();
    let data_result = body
        .ok_or::<Error>("missing body".into()).chain_err(|| "missing body")
        .and_then(|valid_body| serde_urlencoded::from_bytes::<TestTokenInput>(&valid_body.as_bytes())
            .chain_err(|| "could not parse body as TestTokenInput"));

    match data_result {
        Ok(data) => {
            let expires_in = 600;
            let p1 = AuthenticationContext {
                user_id: data.user_id,
                app_id: data.app_id,
            }.build_payload(expires_in);
            let header = Header::new(Algorithm::RS256);

            let tokens = Tokens {
                access_token: encode(header, JWT_SECRET_KEY.to_owned(), p1.clone()),
                refresh_token: None,
                expires_in: expires_in,
            };
            Ok(ApiGatewayResponse {
                status_code: http::StatusCode::OK,
                body: Some((serde_json::to_string(&tokens).unwrap(), mime::APPLICATION_JSON)),
                ..Default::default()
            })
        },
        Err(e) => {
            println!("failed to parse form body ({:?}): {}", body, e);
            let oauth_error = Oauth2Error {
                error: Oauth2ErrorMessage::InvalidRequest,
                error_description: None,
            };
            Ok(ApiGatewayResponse {
                status_code: http::StatusCode::BAD_REQUEST,
                body: Some((serde_json::to_string(&oauth_error).unwrap(), mime::APPLICATION_JSON)),
                ..Default::default()
            })
        }
    }

}

pub fn get_pub_certificate(_event: Value, _context: LambdaContext) -> LambdaResult<ApiGatewayResponse> {
    let mut f = File::open(JWT_PUB_KEY).expect("file not found");

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    Ok(ApiGatewayResponse {
        status_code: http::StatusCode::OK,
        body: Some((contents, mime::TEXT_PLAIN)),
        ..Default::default()
    })
}

fn wrapped_decode_jwt(token: String) -> Result<(Header, Payload)> {
    match ::std::panic::catch_unwind(|| decode(token, JWT_PUB_KEY.to_owned(), Algorithm::RS256)
        .map_err(|err| Error::from(format!("{:?}", err)))
        .chain_err(|| "could not read JWT token")
    ) {
        Ok(result) => result,
        Err(error) => Err(Error::from(format!("error in JWT lib: {:?}", error)))
    }
}

pub fn check_authorization(event: Value, _context: LambdaContext) -> LambdaResult<Policy> {
    let auth_header = event["authorizationToken"].as_str();
    let authentication_context = auth_header
        .ok_or::<Error>("missing header".into()).chain_err(|| "missing header")
        .and_then(|header| match header.starts_with("bearer ") {
            true => Ok(header[7..].to_owned()),
            false => Err(Error::from("authorization is not a bearer token"))
        })
        .and_then(|token| wrapped_decode_jwt(token))
        .and_then(|(_, payload)| AuthenticationContext::try_from_payload(payload));
    match authentication_context {
        Ok(ac) => Ok(Policy::allow_all(String::from("user"), ac.to_hashmap())),
        Err(error) => {
            println!("error during authorization: {:?}", error);
            Err(PyException("Unauthorized".to_owned()).into())
        }
    }
}

#[derive(Debug)]
struct AuthenticationContext {
    user_id: String,
    app_id: String,
}
impl AuthenticationContext {
    pub fn try_from_payload(p: Payload) -> Result<AuthenticationContext> {
        if !p.contains_key("user_id") {
            return Err("missing user_id".into());
        }
        if !p.contains_key("app_id") {
            return Err("missing app_id".into());
        }
        if !p.contains_key("expires_at") {
            return Err("missing expires_at".into());
        }
        match p["expires_at"].parse::<i64>() {
            Ok(expires_at) if expires_at < time::get_time().sec => return Err("token expired".into()),
            _ => ()
        }

        Ok(AuthenticationContext {
            user_id: p["user_id"].to_owned(),
            app_id: p["app_id"].to_owned(),
        })
    }
    pub fn to_hashmap(self) -> HashMap<String, String> {
        let mut hashmap = HashMap::new();
        hashmap.insert("user_id".to_owned(), self.user_id);
        hashmap.insert("app_id".to_owned(), self.app_id);
        hashmap
    }
    pub fn build_payload(self, expires_in: u32) -> Payload {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), self.user_id);
        p.insert("app_id".to_string(), self.app_id);
        p.insert("expires_at".to_string(), (time::get_time().sec + i64::from(expires_in)).to_string());
        p.insert("session_id".to_string(), format!("{}", uuid::Uuid::new_v4().hyphenated()));
        p.insert("token_id".to_string(), format!("{}", uuid::Uuid::new_v4().hyphenated()));
        p
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_transform_a_user_to_payload() {
        let payload = AuthenticationContext {
            user_id: "u1".to_owned(),
            app_id: "a1".to_owned(),
        }.build_payload(5);
        assert_eq!("u1", payload["user_id"])
    }

    #[test]
    fn can_extract_a_user_from_payload() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(), (time::get_time().sec + i64::from(5)).to_string());
        let user = AuthenticationContext::try_from_payload(p);

        assert!(user.is_ok());
    }

    #[test]
    fn should_reject_if_expired() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(), (time::get_time().sec + i64::from(-5)).to_string());
        let user = AuthenticationContext::try_from_payload(p);

        assert!(user.is_err());
        let err = user.unwrap_err();
        assert_eq!(err.description(), "token expired");
        assert_eq!(err.to_string(), "token expired");
    }
}
