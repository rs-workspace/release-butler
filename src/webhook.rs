use actix_web::{
    post,
    web::{self, Bytes},
    HttpRequest, HttpResponse, Responder,
};
use base64ct::Encoding;
use hmac::{Hmac, Mac};
use octocrab::models::Event;
use tracing::{debug, error, info};

// The Webhook Payload size limit is 25MB
pub static WEBHOOK_SIZE_LIMIT: usize = 25_000_000; // 25 * 1000 * 1000

pub type HmacSha256 = Hmac<sha2::Sha256>;

pub struct GitHubSignature256(pub Box<str>);

#[post("/github/webhook/")]
pub async fn parse_event(req: HttpRequest, body: web::Payload) -> impl Responder {
    let headers = req.headers();
    let Some(github_signature_256) = headers.get("X-Hub-Signature-256") else {
        error!("The request on `/github/webhook` didn't contained `X-Hub-Signature-256` header. Complete Response -> {:?}", req);
        return HttpResponse::BadRequest();
    };

    let Ok(body) = body.to_bytes_limited(WEBHOOK_SIZE_LIMIT).await else {
        error!(
            "Body size is greater than 25MB. Complete Response -> {:?}",
            req
        );
        return HttpResponse::InternalServerError();
    };

    let body = match body {
        Ok(bytes) => bytes,
        Err(err) => {
            error!("Failed to convert body payload to bytes. Error: {:?}", err);
            Bytes::new()
        }
    };

    if body.len() == 0 {
        return HttpResponse::InternalServerError();
    }

    let mut hasher = HmacSha256::new_from_slice(
        std::env::var("RELEASE-BUTLER-SECRET")
            .expect("Please provide env variable, `RELEASE-BUTLER-SECRET` which contains GitHub Webhook Secret.")
            .as_bytes()
    ).expect("Failed to create Hasher");
    hasher.update(&body);

    let mut enc_buf = [0u8; 256];
    let Ok(signature_256) = base64ct::Base64::encode(&hasher.finalize().into_bytes(), &mut enc_buf)
    else {
        error!("hmm! InvalidLengthError Insufficient output buffer length.");
        return HttpResponse::InternalServerError();
    };

    if github_signature_256.as_bytes() != signature_256.as_bytes() {
        error!("Invalid Signature. This is not a valid webhook event send by GitHub");
        return HttpResponse::BadRequest();
    }
    // Great, go ahead now it's verified that this is send from GitHub
    let Ok(event) = serde_json::from_slice::<Event>(&body) else {
        error!("Failed to serialize body");
        return HttpResponse::InternalServerError();
    };

    debug!("Got Webhook Event {:#?}", event);

    match event {
        // TODO
        _ => {
            info!("Got an unsupported event: {:?}", event)
        }
    }

    HttpResponse::Ok()
}
