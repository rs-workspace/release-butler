//! This is a tests utility crate, which contains all the necessary type defination and
//! functions required for running tests.
//! See `tests` directory for more information

use super::webhook::parse_event;
pub use crate::webhook::WebhookError;
use crate::State;
use actix_http::{header::TryIntoHeaderPair, Request};
pub use actix_web::http::StatusCode;
pub use actix_web::test;
use actix_web::{dev::ServiceResponse, error::HttpError, web::Data, App};

pub static WEBHOOK_ENDPOINT: &str = "/github/webhook/";
pub static DEFAULT_HMAC_KEY: &str = "abc";

#[derive(Debug, Clone)]
pub struct HeaderConversionError;

impl From<HeaderConversionError> for HttpError {
    fn from(_val: HeaderConversionError) -> HttpError {
        todo!()
    }
}

pub struct TestHeader<S: AsRef<str>>(pub &'static str, pub S);

impl<S> TryIntoHeaderPair for TestHeader<S>
where
    S: AsRef<str>,
{
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
        let header_value = actix_http::header::HeaderValue::from_str(self.1.as_ref());

        match header_value {
            Ok(value) => Ok((header_name, value)),
            Err(_) => Err(HeaderConversionError),
        }
    }
}

pub async fn test_endpoint(req: Request) -> ServiceResponse {
    let app = test::init_service(App::new().service(parse_event).app_data(Data::new(State {
        webhook_secret: String::from(DEFAULT_HMAC_KEY),
    })))
    .await;
    test::call_service(&app, req).await
}

pub mod payload_template {
    use std::sync::LazyLock;

    use super::DEFAULT_HMAC_KEY;
    use crate::common::generate_hmac_sha256_hex;

    pub static GITHUB_PR_OPENED: &[u8] =
        include_str!("../tests_payload/github_pr_opened.json").as_bytes();

    pub static GITHUB_PR_OPENED_HEX: LazyLock<String> = LazyLock::new(|| {
        let mut sha_hex = String::from("sha256=");
        sha_hex.push_str(
            &generate_hmac_sha256_hex(GITHUB_PR_OPENED, DEFAULT_HMAC_KEY.as_bytes())
                .unwrap_or_default(),
        );
        sha_hex
    });

    pub static GITHUB_PUSH: &[u8] = include_str!("../tests_payload/github_push.json").as_bytes();

    pub static GITHUB_PUSH_HEX: LazyLock<String> = LazyLock::new(|| {
        let mut sha_hex = String::from("sha256=");
        sha_hex.push_str(
            &generate_hmac_sha256_hex(GITHUB_PUSH, DEFAULT_HMAC_KEY.as_bytes()).unwrap_or_default(),
        );
        sha_hex
    });

    pub static GITHUB_FORK: &[u8] = include_str!("../tests_payload/github_fork.json").as_bytes();

    pub static GITHUB_FORK_HEX: LazyLock<String> = LazyLock::new(|| {
        let mut sha_hex = String::from("sha256=");
        sha_hex.push_str(
            &generate_hmac_sha256_hex(GITHUB_FORK, DEFAULT_HMAC_KEY.as_bytes()).unwrap_or_default(),
        );
        sha_hex
    });

    pub static GITHUB_ISSUES: &[u8] =
        include_str!("../tests_payload/github_issues.json").as_bytes();

    pub static GITHUB_ISSUES_HEX: LazyLock<String> = LazyLock::new(|| {
        let mut sha_hex = String::from("sha256=");
        sha_hex.push_str(
            &generate_hmac_sha256_hex(GITHUB_ISSUES, DEFAULT_HMAC_KEY.as_bytes())
                .unwrap_or_default(),
        );
        sha_hex
    });

    pub static GITHUB_INVALID_ISSUES_PAYLOAD: &[u8] =
        include_str!("../tests_payload/github_invalid_issues_payload.json").as_bytes();

    pub static GITHUB_INVALID_ISSUES_PAYLOAD_HEX: LazyLock<String> = LazyLock::new(|| {
        let mut sha_hex = String::from("sha256=");
        sha_hex.push_str(
            &generate_hmac_sha256_hex(GITHUB_INVALID_ISSUES_PAYLOAD, DEFAULT_HMAC_KEY.as_bytes())
                .unwrap_or_default(),
        );
        sha_hex
    });
}
