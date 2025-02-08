use super::*;
use octocrab::models::webhook_events::WebhookEvent;

pub struct IssuesHandler<'a> {
    payload: &'a WebhookEvent,
    config: Config,
    state: &'a State,
}

impl<'a> Handler<'a> for IssuesHandler<'a> {
    fn new(payload: &'a WebhookEvent, config: Config, state: &'a State) -> Self {
        Self {
            payload,
            config,
            state,
        }
    }

    fn execute(&self) -> Result<HttpResponse, WebhookError> {
        Ok(HttpResponse::Ok().finish())
    }
}
