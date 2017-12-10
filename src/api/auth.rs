use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use std::ops::Add;

use crowbar::{PyException, Value, LambdaContext, LambdaResult, Policy, ApiGatewayResponse};
use serde_json;
use serde_urlencoded;

use frank_jwt::{Header, Payload, Algorithm, encode, decode};
use uuid;
use time;
use http;
use mime;

use failure::Error;

use model;

use JWT_PUB_KEY;
use JWT_SECRET_KEY;

#[derive(Debug, Fail)]
enum InputError {
    #[fail(display = "Missing Header: '{}'", _0)]
    MissingHeader(String),
    #[fail(display = "Missing Body")]
    MissingBody,
    #[fail(display = "Missing Field: '{}'", _0)]
    MissingField(String),
    #[fail(display = "Invalid Header: '{}'", _0)]
    InvalidHeader { name: String },
    #[fail(display = "Could Not Parse Field: '{}'", _0)]
    ParsingError(String),
    #[fail(display = "Expired Token")]
    ExpiredToken,
}

#[derive(Debug, Fail)]
enum ProcessError {
    #[fail(display = "JWT Error: '{}'", _0)]
    JWTError(String),
}

#[derive(Serialize, Debug, Clone, Default)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
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

pub fn test_token(event: &Value, _context: &LambdaContext) -> LambdaResult<ApiGatewayResponse> {
    let body = event["body"].as_str();
    let data_result = body.ok_or_else(|| InputError::MissingBody)
        .and_then(|valid_body| {
                      serde_urlencoded::from_bytes::<TestTokenInput>(valid_body.as_bytes())
                          .map_err(|_| InputError::ParsingError("body".to_string()))
                  });

    match data_result {
        Ok(data) => {
            let expires_in = time::Duration::days(1);
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
                expires_in: expires_in.num_seconds(),
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

pub fn get_pub_certificate(_event: &Value,
                           _context: &LambdaContext)
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

fn wrapped_decode_jwt(token: String) -> Result<(Header, Payload), ProcessError> {
    match ::std::panic::catch_unwind(|| decode(token, JWT_PUB_KEY.to_owned(), Algorithm::RS256)) {
        Ok(result) => {
            result
                .map_err(|err| ProcessError::JWTError(format!("error decoding the JWT {:?}", err)))
        }
        Err(error) => Err(ProcessError::JWTError(format!("error decoding the JWT {:?}", error))),
    }
}

pub fn check_authorization(event: &Value, _context: &LambdaContext) -> LambdaResult<Policy> {
    let auth_header = event["authorizationToken"].as_str();
    let authentication_context =
        auth_header
            .ok_or_else(|| InputError::MissingHeader("authorization".to_string()))
            .and_then(|header| if header.to_lowercase().starts_with("bearer ") {
                          Ok(header[7..].to_owned())
                      } else {
                          Err(InputError::InvalidHeader { name: "authorization".to_string() })
                      })
            .map_err(|err| err.into())
            .and_then(|token| wrapped_decode_jwt(token).map_err(|err| err.into()))
            .and_then(|(_, payload)| AuthenticationContext::try_from(&payload));
    match authentication_context {
        Ok(ac) => Ok(Policy::allow_all(String::from("user"), ac.to_hashmap())),
        Err(error) => {
            println!("error during authorization: {:?}", error);
            println!("cause: {:?}", error.cause());
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
    pub fn try_from(p: &Payload) -> Result<AuthenticationContext, Error> {
        let user_id = p.get("user_id")
            .ok_or_else(|| InputError::MissingField("user_id".to_string()));
        let user = AuthenticationContext::get_user_from(user_id?)
            .map_err(|err| format_err!("Error code: {}", err))?;

        let app_id = p.get("app_id")
            .ok_or_else(|| InputError::MissingField("app_id".to_string()));

        let expires_at = p.get("expires_at")
            .ok_or_else(|| InputError::MissingField("expires_at".to_string()))
            .and_then(|expires_at| {
                          expires_at
                              .parse::<i64>()
                              .map_err(|_| InputError::ParsingError("expires_at".to_string()))
                      })?;
        if expires_at < time::get_time().sec {
            return Err(InputError::ExpiredToken {})?;
        }

        Ok(AuthenticationContext {
               user: user,
               app_id: app_id?.to_string(),
           })
    }

    fn get_user_from(user_id: &str) -> Result<model::app::User, Error> {
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
    pub fn to_payload(&self, expires_in: time::Duration) -> Payload {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), self.user.user_id.to_string());
        p.insert("app_id".to_string(), self.app_id.to_string());
        p.insert("expires_at".to_string(),
                 (time::get_time().add(expires_in)).sec.to_string());
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
            .to_payload(time::Duration::seconds(57));

        assert_eq!("u1", payload["user_id"])
    }

    #[test]
    fn can_extract_a_user_from_payload() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(),
                 (time::get_time().sec + i64::from(5)).to_string());
        let auth_context = AuthenticationContext::try_from(&p);

        assert!(auth_context.is_ok());
    }

    #[test]
    fn should_reject_if_expired() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(),
                 (time::get_time().sec + i64::from(-5)).to_string());
        let auth_context = AuthenticationContext::try_from(&p);

        assert!(auth_context.is_err());
        let err = auth_context.unwrap_err();
        assert_eq!(err.to_string(), "Expired Token");
    }

    #[test]
    fn should_reject_if_missing_user_id() {
        let mut p = Payload::new();
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(),
                 (time::get_time().sec + i64::from(-5)).to_string());
        let auth_context = AuthenticationContext::try_from(&p);

        assert!(auth_context.is_err());
        let err = auth_context.unwrap_err();
        assert_eq!(err.to_string(), "Missing Field: \'user_id\'");
    }

    #[test]
    fn should_reject_if_unparsable_expired_at() {
        let mut p = Payload::new();
        p.insert("user_id".to_string(), "1".to_owned());
        p.insert("app_id".to_string(), "1".to_owned());
        p.insert("expires_at".to_string(), "AZER".to_string());
        let auth_context = AuthenticationContext::try_from(&p);

        assert!(auth_context.is_err());
        let err = auth_context.unwrap_err();
        assert_eq!(err.to_string(), "Could Not Parse Field: \'expires_at\'");
    }
}
