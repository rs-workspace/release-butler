use crate::{webhook::WebhookError, State};
use actix_web::HttpResponse;
use octocrab::models::webhook_events::WebhookEvent;

pub mod issues;

#[async_trait::async_trait]
pub trait Handler<'a> {
    fn new(repository: (&'a str, &'a str), payload: &'a WebhookEvent, state: &'a State) -> Self;

    async fn execute(&self) -> Result<HttpResponse, WebhookError>;
}
