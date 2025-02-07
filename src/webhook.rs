use actix_web::{
    post,
    web::{self, Bytes},
    HttpRequest, HttpResponse, Responder,
};
use hmac::{Hmac, Mac};
use octocrab::models::webhook_events::WebhookEvent;
use tracing::{error, info};

use crate::State;

// The Webhook Payload size limit is 25MB
pub static WEBHOOK_SIZE_LIMIT: usize = 25_000_000; // 25 * 1000 * 1000

pub type HmacSha256 = Hmac<sha2::Sha256>;

pub struct GitHubSignature256(pub Box<str>);

#[post("/github/webhook/")]
pub async fn parse_event(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<State>,
) -> impl Responder {
    let headers = req.headers();

    let github_signature_256 = match headers.get("X-Hub-Signature-256") {
        Some(value) => {
            let value = value.to_str().unwrap_or("");
            value
        }
        None => {
            error!("The request on `/github/webhook` didn't contained `X-Hub-Signature-256` header. Request -> {:?}", req);
            ""
        }
    };

    let github_event = match headers.get("X-GitHub-Event") {
        Some(value) => {
            let value = value.to_str().unwrap_or("");
            value
        }
        None => {
            error!("Failed to convert value of `X-GitHub-Event` header into str");
            ""
        }
    };

    if github_signature_256.is_empty() || github_event.is_empty() {
        error!("Either the header `X-Hub-Signature-256` or `X-GitHub-Event` was empty or one of them failed to parse");
        return HttpResponse::BadRequest();
    }

    let Ok(body) = body.to_bytes_limited(WEBHOOK_SIZE_LIMIT).await else {
        error!("Body size is greater than 25MB.");
        return HttpResponse::InternalServerError();
    };

    let body = match body {
        Ok(bytes) => bytes,
        Err(err) => {
            error!("Failed to convert body payload to bytes. Error: {:?}", err);
            Bytes::new()
        }
    };

    if body.is_empty() {
        info!("Got empty payload, ignoring the request");
        return HttpResponse::InternalServerError();
    }

    let mut hasher = HmacSha256::new_from_slice(state.webhook_secret.as_bytes())
        .expect("Failed to create Hasher");
    hasher.update(&body);

    let mut enc_buf = [0u8; 256];
    let Ok(signature_256) =
        base16ct::lower::encode_str(&hasher.finalize().into_bytes(), &mut enc_buf)
    else {
        error!("hmm! InvalidLengthError Insufficient output buffer length.");
        return HttpResponse::InternalServerError();
    };
    let signature_256 = format!("sha256={}", signature_256);

    if github_signature_256.as_bytes() != signature_256.as_bytes() {
        error!("Invalid Signature. This is not a valid webhook event send by GitHub. Our signature = {}, header signature = {}", signature_256, github_signature_256);
        return HttpResponse::BadRequest();
    }
    // Great, go ahead now it's verified that this is send from GitHub
    let Ok(event) = WebhookEvent::try_from_header_and_body(github_event, &body) else {
        error!("Failed to serialize webhook payload. body => {:?}", body);
        return HttpResponse::InternalServerError();
    };

    info!("Got Webhook Event {:#?}", event);

    match event {
        // TODO
        _ => {
            info!("Got an unsupported event: {:?}", event)
        }
    }

    HttpResponse::Ok()
}
