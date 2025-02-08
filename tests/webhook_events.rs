use release_butler::tests_utils::*;

#[actix_web::test]
async fn test_unknown_event_header() {
    let signature_header = TestHeader(
        "x-hub-signature-256",
        &*payload_template::GITHUB_PR_OPENED_HEX,
    );
    let event_header = TestHeader("x-github-event", "some-none-existing-event");
    let body = payload_template::GITHUB_PR_OPENED;

    let req = test::TestRequest::post()
        .uri(WEBHOOK_ENDPOINT)
        .set_payload(body)
        .insert_header(signature_header)
        .insert_header(event_header)
        .to_request();

    let resp = test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);

    let body = test::read_body(resp).await;
    assert_eq!(body, WebhookError::UnsupportedEvent.to_bytes())
}

#[actix_web::test]
async fn test_unsubscribed_event() {
    let signature_header = TestHeader("x-hub-signature-256", &*payload_template::GITHUB_FORK_HEX);
    let event_header = TestHeader("x-github-event", "fork");
    let body = payload_template::GITHUB_FORK;

    let req = test::TestRequest::post()
        .uri(WEBHOOK_ENDPOINT)
        .set_payload(body)
        .insert_header(signature_header)
        .insert_header(event_header)
        .to_request();

    let resp = test_endpoint(req).await;

    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);

    let body = test::read_body(resp).await;
    assert_eq!(body, WebhookError::UnsupportedEvent.to_bytes())
}

#[actix_web::test]
async fn test_empty_repository_information() {
    let signature_header = TestHeader(
        "x-hub-signature-256",
        &*payload_template::GITHUB_INVALID_ISSUES_PAYLOAD_HEX,
    );
    let event_header = TestHeader("x-github-event", "issues");
    let body = payload_template::GITHUB_INVALID_ISSUES_PAYLOAD;

    let req = test::TestRequest::post()
        .uri(WEBHOOK_ENDPOINT)
        .set_payload(body)
        .insert_header(signature_header)
        .insert_header(event_header)
        .to_request();

    let resp = test_endpoint(req).await;

    // assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = test::read_body(resp).await;
    assert_eq!(
        body,
        WebhookError::MalformatedBody {
            msg: String::from("Repository Information is required"),
        }
        .to_bytes()
    )
}
