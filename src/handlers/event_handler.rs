use rocket::{post, State, response::status::Accepted};
use rocket::serde::json::Json;
use crate::models::event::EventRequest;
use crate::services::event_service::{EventService, EventServiceError};
use std::sync::Arc;
use tokio::sync::Mutex;

#[post("/api/event", data = "<request>")]
pub async fn handle_event(
    request: Json<EventRequest>,
    service: &State<Arc<Mutex<EventService>>>,
) -> Result<Accepted<()>, rocket::http::Status> {
    let service = service.lock().await;
    
    match service.process_event(request.into_inner()).await {
        Ok(_) => Ok(Accepted(())),
        Err(e) => {
            match e {
                EventServiceError::InvalidEvent(_) => Err(rocket::http::Status::BadRequest),
                EventServiceError::BufferFull => Err(rocket::http::Status::TooManyRequests),
                _ => Err(rocket::http::Status::InternalServerError),
            }
        }
    }
} 