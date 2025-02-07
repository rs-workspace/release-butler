use actix_web::web::ServiceConfig;
use release_butler::webhook;
use shuttle_actix_web::ShuttleActixWeb;

#[shuttle_runtime::main]
async fn main() -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    let config = move |cfg: &mut ServiceConfig| {
        cfg.service(webhook::parse_event);
    };

    Ok(config.into())
}
