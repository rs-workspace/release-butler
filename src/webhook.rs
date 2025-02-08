use crate::{
    common::generate_hmac_sha256_hex,
    State,
};
use actix_web::{
    http::{header::ContentType, StatusCode},
    post,
    web::{self, Bytes},
    HttpRequest, HttpResponse, ResponseError,
};
use derive_more::{Display, Error};
use octocrab::models::webhook_events::{WebhookEvent, WebhookEventType};
use tracing::{error, info};

// The Webhook Payload size limit is 25MB
pub static WEBHOOK_SIZE_LIMIT: usize = 25_000_000; // 25 * 1000 * 1000

pub type HmacSha256 = Hmac<sha2::Sha256>;

#[derive(Debug, Display, Error, Clone, Copy)]
pub enum WebhookError {
    #[display("Not all the required headers are available")]
    RequiredHeadersNotAvailable,
    #[display("Body size greater than WEBHOOK_SIZE_LIMIT i.e. 25MB")]
    LargeBodySize,
    #[display("Empty body/payload")]
    EmptyBody,
    #[display("Signature in X-Hub-Signature-256 and computed from payload didn't matched")]
    InvalidSignature,
    #[display("Failed to serialize the payload")]
    SerializationFailed,
    #[display("Got an unsupported webhook event")]
    UnsupportedEvent,
    #[display("Invalid Encoding or length when computing sha256 signature")]
    InvalidEncodingOrLength,
}

impl WebhookError {
    pub fn to_bytes(self) -> web::Bytes {
        web::Bytes::from(self.to_string())
    }
}

impl ResponseError for WebhookError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .content_type(ContentType::plaintext())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            WebhookError::RequiredHeadersNotAvailable => StatusCode::NOT_ACCEPTABLE,
            WebhookError::LargeBodySize => StatusCode::PAYLOAD_TOO_LARGE,
            WebhookError::EmptyBody => StatusCode::BAD_REQUEST,
            WebhookError::InvalidSignature => StatusCode::UNAUTHORIZED,
            WebhookError::SerializationFailed => StatusCode::INTERNAL_SERVER_ERROR,
            WebhookError::UnsupportedEvent => StatusCode::NOT_IMPLEMENTED,
            WebhookError::InvalidEncodingOrLength => StatusCode::BAD_REQUEST,
        }
    }
}

pub struct GitHubSignature256(pub Box<str>);

#[post("/github/webhook/")]
pub async fn parse_event(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<State>,
) -> Result<HttpResponse, WebhookError> {
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
        return Err(WebhookError::RequiredHeadersNotAvailable);
    }

    let Ok(body) = body.to_bytes_limited(WEBHOOK_SIZE_LIMIT).await else {
        error!("Body size is greater than 25MB.");
        return Err(WebhookError::LargeBodySize);
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
        return Err(WebhookError::MalformatedBody {
            msg: String::from("The payload/body can't be empty"),
        });
    }

    let Some(signature_256) = generate_hmac_sha256_hex(&body, state.webhook_secret.as_bytes())
    else {
        error!("hmm! InvalidLengthError Insufficient output buffer length.");
        return Err(WebhookError::InvalidEncodingOrLength);
    };
    let signature_256 = format!("sha256={}", signature_256);

    if github_signature_256.as_bytes() != signature_256.as_bytes() {
        error!("Invalid Signature. This is not a valid webhook event send by GitHub. Our signature = {}, header signature = {}", signature_256, github_signature_256);
        return Err(WebhookError::InvalidSignature);
    }
    // Great, go ahead now it's verified that this is send from GitHub
    let Ok(event) = WebhookEvent::try_from_header_and_body(github_event, &body) else {
        error!("Failed to serialize webhook payload. body => {:?}", body);
        return Err(WebhookError::SerializationFailed);
    };

    match event.kind {
        // TODO
        _ => {
            info!("Got an unsupported event: {:?}", event);
            return Err(WebhookError::UnsupportedEvent);
        }
    }
}
