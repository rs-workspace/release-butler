use crate::{config::Config, webhook::WebhookError, State};
use actix_web::HttpResponse;
use octocrab::{models::webhook_events::WebhookEvent, Octocrab};

pub mod issues;

pub trait Handler<'a> {
    fn new(payload: &'a WebhookEvent, config: Config, state: &'a State, gh: Octocrab) -> Self;

    fn execute(&self) -> Result<HttpResponse, WebhookError>;
}
