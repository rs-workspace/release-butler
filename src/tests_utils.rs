//! This is a tests utility crate, which contains all the necessary type defination and
//! functions required for running tests.
//! See `tests` directory for more information

use super::webhook::parse_event;
use crate::State;
use actix_http::{header::TryIntoHeaderPair, Request};
pub use actix_web::test;
use actix_web::{dev::ServiceResponse, error::HttpError, web::Data, App};
use std::sync::LazyLock;

pub static WEBHOOK_ENDPOINT: &str = "/github/webhook";
pub static APP_STATE: LazyLock<State> = LazyLock::new(|| State {
    webhook_secret: String::from("abc"),
});

#[derive(Debug, Clone)]
pub struct HeaderConversionError;

impl Into<HttpError> for HeaderConversionError {
    fn into(self) -> HttpError {
        todo!()
    }
}

pub struct TestHeader(pub &'static str, pub Box<str>);

impl TryIntoHeaderPair for TestHeader {
    type Error = HeaderConversionError;

    fn try_into_pair(
        self,
    ) -> Result<
        (
            actix_http::header::HeaderName,
            actix_http::header::HeaderValue,
        ),
        Self::Error,
    > {
        let header_name = actix_http::header::HeaderName::from_static(self.0);
        let header_value = actix_http::header::HeaderValue::from_str(&self.1);

        match header_value {
            Ok(value) => Ok((header_name, value)),
            Err(_) => Err(HeaderConversionError),
        }
    }
}

pub async fn test_endpoint(req: Request) -> ServiceResponse {
    let app = test::init_service(
        App::new()
            .service(parse_event)
            .app_data(Data::new(&APP_STATE)),
    )
    .await;
    test::call_service(&app, req).await
}
