use actix_web::web::{Data, ServiceConfig};
use release_butler::{webhook, State};
use shuttle_actix_web::ShuttleActixWeb;
use shuttle_runtime::SecretStore;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    let webhook_secret = secrets
        .get("RELEASE-BUTLER-SECRET")
        .expect("Please provide secret `RELEASE-BUTLER-SECRET` which contains GitHub Webhook Secret. For more info, refer https://docs.shuttle.dev/resources/shuttle-secrets");

    let config = move |cfg: &mut ServiceConfig| {
        cfg.service(webhook::parse_event)
            .app_data(Data::new(State { webhook_secret }));
    };

    Ok(config.into())
}
