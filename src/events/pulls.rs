use octocrab::models::webhook_events::{
    payload::PullRequestWebhookEventAction, WebhookEventPayload,
};
use tracing::error;

use crate::webhook::generate_gh_from_event;

use super::*;

pub struct PullsHandler<'a> {
    payload: &'a WebhookEvent,
    state: &'a State,
    repository: (&'a str, &'a str),
}

#[async_trait::async_trait]
impl<'a> Handler<'a> for PullsHandler<'a> {
    fn new(repository: (&'a str, &'a str), payload: &'a WebhookEvent, state: &'a State) -> Self {
        Self {
            repository,
            payload,
            state,
        }
    }

    async fn execute(&self) -> Result<HttpResponse, WebhookError> {
        let WebhookEventPayload::PullRequest(pull) = &self.payload.specific else {
            error!("Got an event, with inner payload not an pull_request. Ignoring the event...");
            return Err(WebhookError::MalformatedBody {
                msg: String::from("Inner Payload must be of pull_request event"),
            });
        };

        match pull.action {
            PullRequestWebhookEventAction::Closed => {
                if let Some(pull_label) = &pull.pull_request.head.label {
                    if pull_label.to_lowercase().starts_with(&format!(
                        "{}:release-butler/",
                        self.repository.0.to_lowercase()
                    )) {
                        let Ok(gh) = generate_gh_from_event(self.payload, &self.state.gh) else {
                            error!("Failed to authenticate from github webhook installation id");
                            return Ok(HttpResponse::Ok().finish());
                        };

                        // Check if PR was merged
                        if let Some(merged) = pull.pull_request.merged {
                            if merged {
                                // Create a tag
                                let Some(commit_sha) = &pull.pull_request.merge_commit_sha else {
                                    return Err(WebhookError::MalformatedBody {
                                        msg: String::from(
                                            "The payload must contain `merge_commit_sha`",
                                        ),
                                    });
                                };

                                let Some(pull_title) = &pull.pull_request.title else {
                                    return Err(WebhookError::MalformatedBody {
                                        msg: String::from("The payload must contain `title`"),
                                    });
                                };

                                let Some(tag) = pull_title.strip_prefix("RELEASE ") else {
                                    if let Err(err) = gh
                                        .issues(self.repository.0, self.repository.1)
                                        .create_comment(
                                            pull.number,
                                            "The release title is malformated. It should be in the following format:\n\n`RELEASE <PACKAGE_NAME>@v<VERSION>`"
                                            )
                                        .await
                                    {
                                        error!("Failed to create PR comment. Error: {}", err);
                                    }
                                    return Ok(HttpResponse::Ok().finish());
                                };

                                let Some(tag) = tag.strip_prefix("@") else {
                                    error!("Failed to find the tag for release");
                                    return Ok(HttpResponse::Ok().finish());
                                };

                                if let Err(err) = gh
                                    .repos(self.repository.0, self.repository.1)
                                    .create_ref(
                                        &octocrab::params::repos::Reference::Tag(tag.to_owned()),
                                        commit_sha,
                                    )
                                    .await
                                {
                                    error!("Failed to create tag. Error: {}", err);
                                }

                                // Create the tag
                                return Ok(HttpResponse::Ok().finish());
                            }
                        }

                        // PR is closed, notify user
                        if let Err(err) = gh
                            .issues(self.repository.0, self.repository.1)
                            .create_comment(
                                pull.number,
                                "You should remove the label `release-butler` from the issue that this PR is addressing instead \
                                of manually closing it as this PR will be created again, if there is any activity on the issue. \
                                If this PR was something else, please don't use PR(s) head branch that starts with `release-butler/` \
                                as they are reserved for me."
                                )
                            .await
                        {
                            error!("Failed to create PR comment. Error: {}", err);
                        }
                    }
                }
            }
            _ => {
                return Err(WebhookError::UnsupportedEvent);
            }
        }
        Ok(HttpResponse::Ok().finish())
    }
}
