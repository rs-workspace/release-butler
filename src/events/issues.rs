use super::*;
use octocrab::{models::webhook_events::WebhookEvent, Octocrab};

pub struct IssuesHandler<'a> {
    payload: &'a WebhookEvent,
    config: Config,
    state: &'a State,
    gh: Octocrab,
}

impl<'a> Handler<'a> for IssuesHandler<'a> {
    fn new(payload: &'a WebhookEvent, config: Config, state: &'a State, gh: Octocrab) -> Self {
        Self {
            payload,
            config,
            state,
            gh,
        }
    }

    fn execute(&self) -> Result<HttpResponse, WebhookError> {
        Ok(HttpResponse::Ok().finish())
    }
}
