use release_butler::tests_utils::*;

#[actix_web::test]
async fn test_empty_signature_header() {
    let req = test::TestRequest::post().uri(WEBHOOK_ENDPOINT).to_request();
    let resp = test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);

    let body = test::read_body(resp).await;
    assert_eq!(body, WebhookError::RequiredHeadersNotAvailable.to_bytes());
}

#[actix_web::test]
async fn test_empty_github_event_header() {
    let req = test::TestRequest::post().uri(WEBHOOK_ENDPOINT).to_request();
    let resp = test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);

    let body = test::read_body(resp).await;
    assert_eq!(body, WebhookError::RequiredHeadersNotAvailable.to_bytes());
}

#[actix_web::test]
async fn test_fail_signature_validation() {
    let signature_header = TestHeader("x-hub-signature-256", "sha256=123");

    let event_header = TestHeader("x-github-event", "issues");

    let req = test::TestRequest::post()
        .uri(WEBHOOK_ENDPOINT)
        .set_payload(r"Hello World!")
        .insert_header(signature_header)
        .insert_header(event_header)
        .to_request();
    let resp = test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body = test::read_body(resp).await;
    assert_eq!(body, WebhookError::InvalidSignature.to_bytes());
}

#[actix_web::test]
async fn test_signature_validation() {
    // The default key is "abc"
    let signature_header = TestHeader(
        "x-hub-signature-256",
        "sha256=2299e6c07452bec21c4b8c341de2052b60571d52e1df6c938a9c49d6dad95111",
    ); // Computed at https://www.devglan.com/online-tools/hmac-sha256-online

    let event_header = TestHeader("x-github-event", "issues");

    let req = test::TestRequest::post()
        .uri(WEBHOOK_ENDPOINT)
        .set_payload(r"Hello World!")
        .insert_header(signature_header)
        .insert_header(event_header)
        .to_request();
    let resp = test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = test::read_body(resp).await;
    assert_eq!(body, WebhookError::SerializationFailed.to_bytes());
}
