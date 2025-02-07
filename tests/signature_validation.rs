use actix_web::http::StatusCode;
use release_butler::{tests_utils, webhook::WebhookError};

#[actix_web::test]
async fn test_empty_signature_header() {
    let req = tests_utils::test::TestRequest::post()
        .uri(tests_utils::WEBHOOK_ENDPOINT)
        .to_request();
    let resp = tests_utils::test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);

    let body = tests_utils::test::read_body(resp).await;
    assert_eq!(body, WebhookError::RequiredHeadersNotAvailable.to_bytes());
}

#[actix_web::test]
async fn test_empty_github_event_header() {
    let req = tests_utils::test::TestRequest::post()
        .uri(tests_utils::WEBHOOK_ENDPOINT)
        .to_request();
    let resp = tests_utils::test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);

    let body = tests_utils::test::read_body(resp).await;
    assert_eq!(body, WebhookError::RequiredHeadersNotAvailable.to_bytes());
}

#[actix_web::test]
async fn test_signature_validation() {
    // The default key is "abc"
    let signature_header = tests_utils::TestHeader(
        "x-hub-signature-256",
        "sha256=2299e6c07452bec21c4b8c341de2052b60571d52e1df6c938a9c49d6dad95111".into(),
    ); // Computed at https://www.devglan.com/online-tools/hmac-sha256-online

    let event_header = tests_utils::TestHeader("x-github-event", "issues".into());

    let req = tests_utils::test::TestRequest::post()
        .uri(tests_utils::WEBHOOK_ENDPOINT)
        .set_payload(r"Hello World!")
        .insert_header(signature_header)
        .insert_header(event_header)
        .to_request();
    let resp = tests_utils::test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = tests_utils::test::read_body(resp).await;
    assert_eq!(body, WebhookError::SerializationFailed.to_bytes());
}
