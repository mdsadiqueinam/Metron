use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: Uuid,
    pub site_id: u64,
    pub user_id: u64,
    pub hostname: String,
    pub timestamp: DateTime<Utc>,
    pub start: DateTime<Utc>,
    pub is_bounce: bool,
    pub entry_page: String,
    pub exit_page: Option<String>,
    pub pageviews: i32,
    pub events: i32,
    pub duration: u32,
    pub referrer: Option<String>,
    pub referrer_source: Option<String>,
    pub country_code: Option<String>,
    pub screen_size: Option<String>,
    pub operating_system: Option<String>,
    pub browser: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_source: Option<String>,
    pub utm_campaign: Option<String>,
    pub browser_version: Option<String>,
    pub operating_system_version: Option<String>,
    pub subdivision1_code: Option<String>,
    pub subdivision2_code: Option<String>,
    pub city_geoname_id: Option<u32>,
    pub utm_content: Option<String>,
    pub utm_term: Option<String>,
    pub entry_meta: Option<SessionMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub keys: Vec<String>,
    pub values: Vec<String>,
}

impl Session {
    pub fn new(site_id: u64, user_id: u64, hostname: String, entry_page: String) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4(),
            site_id,
            user_id,
            hostname,
            timestamp: now,
            start: now,
            is_bounce: true, // Initially set as bounce, updated when more pageviews occur
            entry_page,
            exit_page: None,
            pageviews: 1,
            events: 1,
            duration: 0,
            referrer: None,
            referrer_source: None,
            country_code: None,
            screen_size: None,
            operating_system: None,
            browser: None,
            utm_medium: None,
            utm_source: None,
            utm_campaign: None,
            browser_version: None,
            operating_system_version: None,
            subdivision1_code: None,
            subdivision2_code: None,
            city_geoname_id: None,
            utm_content: None,
            utm_term: None,
            entry_meta: None,
        }
    }

    pub fn add_pageview(&mut self) {
        self.pageviews += 1;
        if self.pageviews > 1 {
            self.is_bounce = false;
        }
    }

    pub fn add_event(&mut self) {
        self.events += 1;
    }

    pub fn update_duration(&mut self) {
        let now = Utc::now();
        self.duration = (now - self.start).num_seconds() as u32;
    }

    pub fn set_exit_page(&mut self, page: String) {
        self.exit_page = Some(page);
    }

    pub fn set_meta(&mut self, keys: Vec<String>, values: Vec<String>) {
        if keys.len() == values.len() {
            self.entry_meta = Some(SessionMeta { keys, values });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let session = Session::new(1, 1, "example.com".to_string(), "/home".to_string());
        assert_eq!(session.site_id, 1);
        assert_eq!(session.user_id, 1);
        assert_eq!(session.hostname, "example.com");
        assert_eq!(session.entry_page, "/home");
        assert_eq!(session.pageviews, 1);
        assert_eq!(session.events, 1);
        assert!(session.is_bounce);
    }

    #[test]
    fn test_add_pageview() {
        let mut session = Session::new(1, 1, "example.com".to_string(), "/home".to_string());
        session.add_pageview();
        assert_eq!(session.pageviews, 2);
        assert!(!session.is_bounce);
    }

    #[test]
    fn test_add_event() {
        let mut session = Session::new(1, 1, "example.com".to_string(), "/home".to_string());
        session.add_event();
        assert_eq!(session.events, 2);
    }

    #[test]
    fn test_set_exit_page() {
        let mut session = Session::new(1, 1, "example.com".to_string(), "/home".to_string());
        session.set_exit_page("/about".to_string());
        assert_eq!(session.exit_page, Some("/about".to_string()));
    }

    #[test]
    fn test_set_meta() {
        let mut session = Session::new(1, 1, "example.com".to_string(), "/home".to_string());
        let keys = vec!["key1".to_string(), "key2".to_string()];
        let values = vec!["value1".to_string(), "value2".to_string()];
        session.set_meta(keys.clone(), values.clone());
        
        if let Some(meta) = session.entry_meta {
            assert_eq!(meta.keys, keys);
            assert_eq!(meta.values, values);
        } else {
            panic!("Meta should be set");
        }
    }
} 