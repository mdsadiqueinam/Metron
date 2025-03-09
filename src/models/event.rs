use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub domain: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
    pub session_id: Option<Uuid>,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub props: Option<serde_json::Value>,
    pub revenue: Option<f64>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventRequest {
    pub name: String,
    pub url: String,
    pub domain: String,
    pub props: Option<serde_json::Value>,
    pub user_id: Option<String>,
    pub revenue: Option<f64>,
}

impl Event {
    pub fn new(request: EventRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: request.name,
            url: request.url,
            domain: request.domain,
            timestamp: Utc::now(),
            user_id: request.user_id,
            session_id: None,
            referrer: None,
            user_agent: None,
            ip_address: None,
            country: None,
            city: None,
            props: request.props,
            revenue: request.revenue,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
        }
    }
} 