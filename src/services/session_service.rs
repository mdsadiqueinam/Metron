use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use anyhow::Result;

use crate::models::session::Session;

const SESSION_TIMEOUT_MINUTES: i64 = 30;

#[derive(Debug, Clone)]
pub struct SessionService {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
}

impl SessionService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        site_id: u64,
        user_id: u64,
        hostname: String,
        entry_page: String,
        referrer: Option<String>,
        user_agent: Option<String>,
        screen_size: Option<String>,
        country_code: Option<String>,
        utm_params: Option<UtmParams>,
    ) -> Result<Session> {
        let mut session = Session::new(site_id, user_id, hostname, entry_page);

        // Set optional parameters
        if let Some(ref_url) = referrer {
            session.referrer = Some(ref_url.clone());
            session.referrer_source = extract_referrer_source(&ref_url);
        }

        if let Some(agent) = user_agent {
            let (browser, browser_version, os, os_version) = parse_user_agent(&agent);
            session.browser = Some(browser);
            session.browser_version = Some(browser_version);
            session.operating_system = Some(os);
            session.operating_system_version = Some(os_version);
        }

        session.screen_size = screen_size;
        session.country_code = country_code;

        if let Some(utm) = utm_params {
            session.utm_medium = utm.medium;
            session.utm_source = utm.source;
            session.utm_campaign = utm.campaign;
            session.utm_content = utm.content;
            session.utm_term = utm.term;
        }

        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id, session.clone());
        
        Ok(session)
    }

    pub async fn get_session(&self, session_id: &Uuid) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn update_session(
        &self,
        session_id: &Uuid,
        page: String,
        event_type: EventType,
    ) -> Result<Session> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            match event_type {
                EventType::Pageview => {
                    session.add_pageview();
                    session.set_exit_page(page);
                }
                EventType::CustomEvent => {
                    session.add_event();
                }
            }
            session.update_duration();
            Ok(session.clone())
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }

    pub async fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        let timeout = Duration::minutes(SESSION_TIMEOUT_MINUTES);
        
        let expired: Vec<_> = sessions
            .iter()
            .filter(|(_, session)| {
                now - session.timestamp > timeout
            })
            .map(|(id, _)| *id)
            .collect();

        let count = expired.len();
        for id in expired {
            sessions.remove(&id);
        }
        
        count
    }
}

#[derive(Debug, Clone)]
pub struct UtmParams {
    pub medium: Option<String>,
    pub source: Option<String>,
    pub campaign: Option<String>,
    pub content: Option<String>,
    pub term: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Pageview,
    CustomEvent,
}

fn extract_referrer_source(referrer: &str) -> Option<String> {
    // Simple referrer source extraction
    // In a real implementation, this would be more sophisticated
    if referrer.contains("google.") {
        Some("Google".to_string())
    } else if referrer.contains("facebook.") {
        Some("Facebook".to_string())
    } else if referrer.contains("twitter.") {
        Some("Twitter".to_string())
    } else {
        None
    }
}

fn parse_user_agent(user_agent: &str) -> (String, String, String, String) {
    // This is a simplified version
    // In a real implementation, you would use a proper user-agent parsing library
    let default = "Unknown".to_string();
    
    if user_agent.contains("Firefox") {
        (
            "Firefox".to_string(),
            "1.0".to_string(),
            "Unknown".to_string(),
            "Unknown".to_string(),
        )
    } else if user_agent.contains("Chrome") {
        (
            "Chrome".to_string(),
            "1.0".to_string(),
            "Unknown".to_string(),
            "Unknown".to_string(),
        )
    } else {
        (
            default.clone(),
            default.clone(),
            default.clone(),
            default,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let service = SessionService::new();
        let session = service.create_session(
            1,
            1,
            "example.com".to_string(),
            "/home".to_string(),
            None,
            None,
            None,
            None,
            None,
        ).await.unwrap();

        assert_eq!(session.site_id, 1);
        assert_eq!(session.user_id, 1);
        assert_eq!(session.hostname, "example.com");
        assert_eq!(session.entry_page, "/home");
    }

    #[tokio::test]
    async fn test_update_session() {
        let service = SessionService::new();
        let session = service.create_session(
            1,
            1,
            "example.com".to_string(),
            "/home".to_string(),
            None,
            None,
            None,
            None,
            None,
        ).await.unwrap();

        let updated = service.update_session(
            &session.session_id,
            "/about".to_string(),
            EventType::Pageview,
        ).await.unwrap();

        assert_eq!(updated.pageviews, 2);
        assert_eq!(updated.exit_page, Some("/about".to_string()));
        assert!(!updated.is_bounce);
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let service = SessionService::new();
        
        // Create a session
        let session = service.create_session(
            1,
            1,
            "example.com".to_string(),
            "/home".to_string(),
            None,
            None,
            None,
            None,
            None,
        ).await.unwrap();

        // Manually set the timestamp to be older than the timeout
        {
            let mut sessions = service.sessions.write().await;
            if let Some(session) = sessions.get_mut(&session.session_id) {
                session.timestamp = Utc::now() - Duration::minutes(SESSION_TIMEOUT_MINUTES + 1);
            }
        }

        let cleaned = service.cleanup_expired_sessions().await;
        assert_eq!(cleaned, 1);

        let session = service.get_session(&session.session_id).await;
        assert!(session.is_none());
    }
} 