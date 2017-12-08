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

use model;

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
            ParseIntError(::std::num::ParseIntError);
        }
        errors {
            MissingHeader(name: &'static str) {
                description("Missing Header")
                display("Missing Header: '{}'", name)
            }
            MissingBody {
                description("Missing Body")
                display("Missing Body")
            }
            MissingField(name: &'static str) {
                description("Missing Field")
                display("Missing Field: '{}'", name)
            }
            InvalidHeader(name: &'static str) {
                description("Invalid Header")
                display("Invalid Header: '{}'", name)
            }
            ExpiredToken {
                description("Expired Token")
                display("Expired Token")
            }
            ParsingError(field: &'static str) {
                description("Parsing Error")
                display("Could Not Parse Field: '{}'", field)
            }
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
    let body = event["body"].as_str();
    let data_result = body.chain_err(|| ErrorKind::MissingBody)
        .and_then(|valid_body| {
                      serde_urlencoded::from_bytes::<TestTokenInput>(valid_body.as_bytes())
                          .chain_err(|| ErrorKind::ParsingError("body"))
                  });

    match data_result {
        Ok(data) => {
            let expires_in = 86400;
            let p1 = AuthenticationContext {
                    user: model::app::User {
                        user_id: model::app::UserId(data.user_id),
                        email: "testemail".to_string(),
                        tz: None,
                    },
                    app_id: data.app_id,
                }
                .to_payload(expires_in);
            let header = Header::new(Algorithm::RS256);

            let tokens = Tokens {
                access_token: encode(header, JWT_SECRET_KEY.to_owned(), p1.clone()),
                refresh_token: None,
                expires_in: expires_in,
            };
            Ok(ApiGatewayResponse {
                   status_code: http::StatusCode::OK,
                   body: Some((Ok(serde_json::to_string(&tokens).unwrap()),
                               mime::APPLICATION_JSON)),
                   ..Default::default()
               })
        }
        Err(e) => {
            println!("failed to parse form body ({:?}): {}", body, e);
            let oauth_error = Oauth2Error {
                error: Oauth2ErrorMessage::InvalidRequest,
                error_description: None,
            };
            Ok(ApiGatewayResponse {
                   status_code: http::StatusCode::BAD_REQUEST,
                   body: Some((Err(serde_json::to_string(&oauth_error).unwrap()),
                               mime::APPLICATION_JSON)),
                   ..Default::default()
               })
        }
    }

}

pub fn get_pub_certificate(_event: Value,
                           _context: LambdaContext)
                           -> LambdaResult<ApiGatewayResponse> {
    let mut f = File::open(JWT_PUB_KEY).expect("file not found");

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    Ok(ApiGatewayResponse {
           status_code: http::StatusCode::OK,
           body: Some((Ok(contents), mime::TEXT_PLAIN)),
           ..Default::default()
       })
}

fn wrapped_decode_jwt(token: String) -> Result<(Header, Payload)> {
    match ::std::panic::catch_unwind(|| decode(token, JWT_PUB_KEY.to_owned(), Algorithm::RS256)) {
        Ok(result) => {
            result.map_err(|err| Error::from(format!("error decoding the JWT {:?}", err)))
        }
        Err(error) => Err(Error::from(format!("error in JWT lib: {:?}", error))),
    }
}

pub fn check_authorization(event: Value, _context: LambdaContext) -> LambdaResult<Policy> {
    let auth_header = event["authorizationToken"].as_str();
    let authentication_context =
        auth_header
            .chain_err(|| ErrorKind::MissingHeader("authorization"))
            .and_then(|header| if header.to_lowercase().starts_with("bearer ") {
                          Ok(header[7..].to_owned())
                      } else {
                          Err(ErrorKind::InvalidHeader("authorization").into())
                      })
            .and_then(wrapped_decode_jwt)
            .and_then(|(_, payload)| AuthenticationContext::try_from(payload));
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
    user: model::app::User,
    app_id: String,
}
impl AuthenticationContext {
    pub fn try_from(p: Payload) -> Result<AuthenticationContext> {
        let user_id = p.get("user_id")
            .chain_err(|| ErrorKind::MissingField("user_id"))?;
        let user = AuthenticationContext::get_user_from(user_id)?;

        let app_id = p.get("app_id")
            .chain_err(|| ErrorKind::MissingField("app_id"))?;

        let expires_at = p.get("expires_at")
            .chain_err(|| ErrorKind::MissingField("expires_at"))
            .and_then(|expires_at| {
                          expires_at
                              .parse::<i64>()
                              .chain_err(|| ErrorKind::ParsingError("expires_at"))
                      })?;
        if expires_at < time::get_time().sec {
            return Err(ErrorKind::ExpiredToken.into());
        }

        Ok(AuthenticationContext {
               user: user,
               app_id: app_id.to_string(),
           })
    }

    fn get_user_from(user_id: &str) -> Result<model::app::User> {
        //TODO: check that user exists and return it from
        Ok(model::app::User {
               user_id: model::app::UserId(user_id.to_string()),
               email: "testemail".to_string(),
               tz: None,
           })
    }
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let mut hashmap = HashMap::new();
        hashmap.insert("user_id".to_string(), self.user.user_id.to_string());
        //TODO: put whole user in hashmap ?
        hashmap.insert("app_id".to_string(), self.app_id.to_owned());
        hashmap
    }
    pub fn to_payload(self, expires_in: u32) -> Payload {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), self.user.user_id.to_string());
        p.insert("app_id".to_string(), self.app_id);
        p.insert("expires_at".to_string(),
                 (time::get_time().sec + i64::from(expires_in)).to_string());
        p.insert("session_id".to_string(),
                 format!("{}", uuid::Uuid::new_v4().hyphenated()));
        p.insert("token_id".to_string(),
                 format!("{}", uuid::Uuid::new_v4().hyphenated()));
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_transform_a_user_to_payload() {
        let payload = AuthenticationContext {
                user: model::app::User {
                    user_id: model::app::UserId("u1".to_string()),
                    email: "testemail".to_string(),
                    tz: None,
                },
                app_id: "a1".to_owned(),
            }
            .to_payload(57);

        assert_eq!("u1", payload["user_id"])
    }

    #[test]
    fn can_extract_a_user_from_payload() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(),
                 (time::get_time().sec + i64::from(5)).to_string());
        let auth_context = AuthenticationContext::try_from(p);

        assert!(auth_context.is_ok());
    }

    #[test]
    fn should_reject_if_expired() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(),
                 (time::get_time().sec + i64::from(-5)).to_string());
        let auth_context = AuthenticationContext::try_from(p);

        assert!(auth_context.is_err());
        let err = auth_context.unwrap_err();
        assert_eq!(err.description(), "Expired Token");
        assert_eq!(err.to_string(), "Expired Token");
    }

    #[test]
    fn should_reject_if_missing_user_id() {
        let mut p = Payload::new();
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(),
                 (time::get_time().sec + i64::from(-5)).to_string());
        let auth_context = AuthenticationContext::try_from(p);

        assert!(auth_context.is_err());
        let err = auth_context.unwrap_err();
        assert_eq!(err.description(), "Missing Field");
        assert_eq!(err.to_string(), "Missing Field: \'user_id\'");
    }

    #[test]
    fn should_reject_if_unparsable_expired_at() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(), "AZER".to_string());
        let auth_context = AuthenticationContext::try_from(p);

        assert!(auth_context.is_err());
        let err = auth_context.unwrap_err();
        assert_eq!(err.description(), "Parsing Error");
        assert_eq!(err.to_string(), "Could Not Parse Field: \'expires_at\'");
    }
}
