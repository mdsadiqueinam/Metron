mod models;
mod handlers;
mod services;
mod config;

use rocket::{launch, Rocket};
use std::sync::Arc;
use tokio::sync::Mutex;
use services::event_service::EventService;
use config::get_config;

#[launch]
async fn rocket() -> Rocket<rocket::Build> {
    // Initialize configuration
    let config = get_config();

    // Initialize event service
    let event_service = EventService::new(config)
        .await
        .expect("Failed to initialize event service");

    // Create shared state
    let service_state = Arc::new(Mutex::new(event_service));

    // Build and configure Rocket
    rocket::build()
        .manage(service_state)
        .mount("/", routes![handlers::event_handler::handle_event])
}
