use crate::{webhook::WebhookError, State};
use actix_web::HttpResponse;
use octocrab::models::webhook_events::WebhookEvent;

pub mod issues;

pub trait Handler<'a> {
    fn new(payload: WebhookEvent, state: &'a State) -> Self;

    fn execute(&self) -> Result<HttpResponse, WebhookError>;
}
