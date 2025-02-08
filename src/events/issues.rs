use super::*;
use crate::State;
use octocrab::models::webhook_events::WebhookEvent;
use tracing::error;

pub struct IssuesHandler<'a> {
    payload: WebhookEvent,
    state: &'a State,
}

impl<'a> Handler<'a> for IssuesHandler<'a> {
    fn new(payload: WebhookEvent, state: &'a State) -> Self {
        Self { payload, state }
    }

    fn execute(&self) -> Result<HttpResponse, WebhookError> {
        let Some(repo) = &self.payload.repository else {
            error!("The payload didn't contains Repository Information. Ignoring the event.");
            return Err(WebhookError::MalformatedBody {
                msg: String::from("Repository Information is required"),
            });
        };

        Ok(HttpResponse::Ok().finish())
    }
}
