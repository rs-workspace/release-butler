use crate::{
    common::generate_hmac_sha256_hex,
    config::Config,
    events::{self, Handler},
    State, DEFAULT_CONFIG_FILE_PATH,
};
use actix_web::{
    http::{header::ContentType, StatusCode},
    post,
    web::{self, Bytes},
    HttpRequest, HttpResponse, ResponseError,
};
use derive_more::{Display, Error};
use octocrab::{
    models::{
        repos::ContentItems,
        webhook_events::{EventInstallation, WebhookEvent, WebhookEventType},
    },
    Octocrab,
};
use tracing::{error, info};

// The Webhook Payload size limit is 25MB
pub static WEBHOOK_SIZE_LIMIT: usize = 25_000_000; // 25 * 1000 * 1000

#[derive(Debug, Display, Error, Clone)]
pub enum WebhookError {
    #[display("Not all the required headers are available")]
    RequiredHeadersNotAvailable,
    #[display("Body size greater than WEBHOOK_SIZE_LIMIT i.e. 25MB")]
    LargeBodySize,
    #[display("The format of body/payload is incorrect. {msg}")]
    MalformatedBody { msg: String },
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
            WebhookError::MalformatedBody { .. } => StatusCode::BAD_REQUEST,
            WebhookError::InvalidSignature => StatusCode::UNAUTHORIZED,
            WebhookError::SerializationFailed => StatusCode::INTERNAL_SERVER_ERROR,
            WebhookError::UnsupportedEvent { .. } => StatusCode::NOT_IMPLEMENTED,
            WebhookError::InvalidEncodingOrLength => StatusCode::BAD_REQUEST,
        }
    }
}

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

    let Some(repository) = &event.repository else {
        error!("The payload didn't contains Repository Information. Ignoring the event.");
        return Err(WebhookError::MalformatedBody {
            msg: String::from("Repository Information is required"),
        });
    };

    let Some(repository) = &repository.full_name else {
        error!("The payload didn't contains repository full name. Ignoring the event.");
        return Err(WebhookError::MalformatedBody {
            msg: String::from("Repository Full Name is required"),
        });
    };

    let repository: Vec<&str> = repository.split("/").collect();
    let repository = (repository[0], repository[1]);

    match &event.kind {
        WebhookEventType::Issues => {
            events::issues::IssuesHandler::new(repository, &event, &state)
                .execute()
                .await
        }
        WebhookEventType::PullRequest => {
            events::pulls::PullsHandler::new(repository, &event, &state)
                .execute()
                .await
        }
        _ => {
            info!("Got an unsupported event: {:?}", event);
            Err(WebhookError::UnsupportedEvent)
        }
    }
}

pub fn generate_gh_from_event(
    event: &WebhookEvent,
    old_gh: &Octocrab,
) -> Result<Octocrab, WebhookError> {
    // Use installation provided by the event
    let Some(event_installation) = &event.installation else {
        error!("The payload didn't contained installation information. Ignoring Event...");
        return Err(WebhookError::MalformatedBody {
            msg: String::from("Installation information is requried."),
        });
    };

    // get installation id
    let installation_id = match event_installation {
        EventInstallation::Full(full) => full.id,
        EventInstallation::Minimal(minimal) => minimal.id,
    };

    old_gh.installation(installation_id).map_err(|err| {
        error!(
            "Failed to generate new Octocrab with installation id provided by event. Error: {}",
            err
        );
        WebhookError::MalformatedBody {
            msg: format!("Failed to new instance of octocrab. Error: {}", err),
        }
    })
}

pub async fn get_config(
    repo_owner: &str,
    repo: &str,
    state: &State,
    gh: &Octocrab,
) -> Option<Config> {
    let repos_handle = gh.repos(repo_owner, repo);

    let config_files = match repos_handle
        .get_content()
        .path(DEFAULT_CONFIG_FILE_PATH)
        .send()
        .await
    {
        Ok(content) => Some(content),
        Err(err) => match err {
            octocrab::Error::GitHub { source, .. } => {
                if source.status_code.as_u16() == 404 {
                    Some(ContentItems { items: Vec::new() })
                } else {
                    None
                }
            }
            _ => {
                error!(
                    "Failed to get reponse from github api when trying to find `{}`. Error: {:?}",
                    DEFAULT_CONFIG_FILE_PATH, err
                );
                None
            }
        },
    }?;

    let mut config_file = String::new();
    for file in config_files.items {
        if file.path.as_str() != DEFAULT_CONFIG_FILE_PATH {
            continue;
        }

        if let Some(file_content) = file.decoded_content() {
            config_file.push_str(&file_content);
        }
    }

    let Ok(config) = toml::from_str::<Config>(&config_file) else {
        error!("Failed to parse configuration file. Posting error as an issue if not exists...");

        let issues = gh.issues(repo_owner, repo);

        // Check if issue already exists or not with label `$CONFIG_ISSUE_LABEL`
        let Ok(issues_list) = issues
            .list()
            .creator(&state.app_username)
            .labels(&[String::from(crate::CONFIG_ISSUE_LABEL)])
            .send()
            .await
        else {
            error!(
                "Failed to get information if issue with label {} by user {} was created or not.",
                crate::CONFIG_ISSUE_LABEL,
                state.app_username
            );
            return None;
        };

        if issues_list.items.is_empty() {
            info!(
                "There is no issue created with label {} by user {}, creating one...",
                crate::CONFIG_ISSUE_LABEL,
                state.app_username
            );

            match issues
                .create(format!("`{}` file is malformatted", DEFAULT_CONFIG_FILE_PATH))
                .body(
                    format!(
                        "Hi there, I just a webhook event for this repository and I failed to get information from `{}`.\n\
                        It is possible that this file doesn't exists or there is an issue with it. Please fix it as it will allow me to work smoothly.\n\n\
                        For more information refer https://github.com/rs-workspace/release-butler\nSample File https://github.com/rs-workspace/release-butler/blob/main/repository.template.toml",
                        DEFAULT_CONFIG_FILE_PATH
                    )
                )
                .labels(vec![String::from(crate::CONFIG_ISSUE_LABEL)])
                .send().await {
                    Ok(_) => {
                        info!("Created an issue highlighting problem with {} in {}/{}", DEFAULT_CONFIG_FILE_PATH, repo_owner, repo);
                    },
                    Err(err) => {
                        error!("Failed to create issue. Error: {:?}", err)
                    }
                }
        }

        return None;
    };

    Some(config)
}
