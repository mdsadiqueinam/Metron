use crate::config::{spam_rules::SpamRulesConfig, Config};
use crate::models::event::{Event, EventRequest};
use clickhouse_rs::Pool;
use futures::TryFutureExt;
use maxminddb::geoip2;
use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::Arc;
use rocket::yansi::Paint;
use thiserror::Error;
use tokio::sync::Mutex;
use url::Url;
use user_agent_parser::UserAgentParser;
use std::borrow::Cow;

#[derive(Error, Debug)]
pub enum EventServiceError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Buffer full")]
    BufferFull,
    #[error("Invalid event: {0}")]
    InvalidEvent(String),
    #[error("Blocked by shield rule: {0}")]
    ShieldRuleBlocked(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Geolocation error: {0}")]
    GeolocationError(String),
    #[error("User agent parsing error: {0}")]
    UserAgentError(String),
}

pub struct EventService {
    config: Arc<Config>,
    pool: Pool,
    buffer: Arc<Mutex<VecDeque<Event>>>,
    geoip_reader: maxminddb::Reader<Vec<u8>>,
    ua_parser: UserAgentParser,
    spam_rules: SpamRulesConfig,
}

impl EventService {
    pub async fn new(config: Config) -> Result<Self, EventServiceError> {
        let pool = Pool::new(config.database_url.clone())
            .map_err(|e| EventServiceError::DatabaseError(e.to_string()))?;

        // Initialize GeoIP reader
        let geoip_reader = maxminddb::Reader::open_readfile(&config.geoip_db_path.clone())
            .map_err(|e| EventServiceError::GeolocationError(e.to_string()))?;

        // Initialize User Agent parser
        let ua_parser = UserAgentParser::new(&config.ua_regexes_path)
            .map_err(|e| EventServiceError::UserAgentError(e.to_string()))?;

        // Load spam rules configuration
        let spam_rules = SpamRulesConfig::load("config/spam_rules.yaml")
            .map_err(|e| EventServiceError::InvalidEvent(format!("Failed to load spam rules: {}", e)))?;

        // Get buffer size before moving config
        let buffer_size = config.max_buffer_size;

        Ok(Self {
            config: Arc::new(config),
            pool,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(buffer_size))),
            geoip_reader,
            ua_parser,
            spam_rules,
        })
    }

    pub async fn process_event(&self, request: EventRequest) -> Result<(), EventServiceError> {
        let mut event = Event::new(request);
        
        // Process pipeline steps
        self.drop_verification_agent(&event)?;
        self.drop_datacenter_ip(&event)?;
        self.drop_spam_referrer(&event)?;
        self.drop_shield_rule_hostname(&event)?;
        self.drop_shield_rule_page(&event)?;
        self.drop_shield_rule_ip(&event)?;
        self.put_geolocation(&mut event)?;
        self.drop_shield_rule_country(&event)?;
        self.put_user_agent(&mut event)?;
        self.put_referrer(&mut event)?;
        self.put_utm_tags(&mut event)?;
        self.validate_event(&event)?;
        self.register_session(&mut event).await?;
        self.buffer_event(event).await?;

        Ok(())
    }

    fn drop_verification_agent(&self, event: &Event) -> Result<(), EventServiceError> {
        if let Some(ua) = &event.user_agent {
            if self.spam_rules.is_bot_user_agent(ua) {
                return Err(EventServiceError::ShieldRuleBlocked("Bot detected".to_string()));
            }
        }
        Ok(())
    }

    fn drop_datacenter_ip(&self, event: &Event) -> Result<(), EventServiceError> {
        // TODO: Implement datacenter IP detection
        Ok(())
    }

    fn drop_shield_rule_hostname(&self, event: &Event) -> Result<(), EventServiceError> {
        // TODO: Implement hostname shield rules
        Ok(())
    }

    fn drop_shield_rule_page(&self, event: &Event) -> Result<(), EventServiceError> {
        if let Ok(url) = Url::parse(&event.url) {
            if self.spam_rules.is_protected_path(url.path()) {
                return Err(EventServiceError::ShieldRuleBlocked("Protected page".to_string()));
            }
        }
        Ok(())
    }

    fn drop_shield_rule_ip(&self, event: &Event) -> Result<(), EventServiceError> {
        if let Some(ip_str) = &event.ip_address {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                match ip {
                    IpAddr::V4(ipv4) => {
                        if ipv4.is_private() || ipv4.is_loopback() {
                            return Err(EventServiceError::ShieldRuleBlocked("Private/Loopback IPv4".to_string()));
                        }
                    },
                    IpAddr::V6(ipv6) => {
                        if ipv6.is_loopback() {
                            return Err(EventServiceError::ShieldRuleBlocked("Loopback IPv6".to_string()));
                        }
                        
                        // Check for Unique Local Address (fc00::/7)
                        let first_byte = ipv6.octets()[0];
                        if first_byte & 0xfe == 0xfc {
                            return Err(EventServiceError::ShieldRuleBlocked("Private IPv6 (ULA)".to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn put_geolocation(&self, event: &mut Event) -> Result<(), EventServiceError> {
        if let Some(ip_str) = &event.ip_address {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                if let Ok(city) = self.geoip_reader.lookup::<geoip2::City>(ip) {
                    if let Some(country) = city.country.and_then(|c| c.iso_code) {
                        event.country = Some(country.to_string());
                    }
                    if let Some(city_name) = city.city.and_then(|c| c.names.and_then(|n| n.get("en"))) {
                        event.city = Some(city_name.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn drop_shield_rule_country(&self, event: &Event) -> Result<(), EventServiceError> {
        if let Some(country) = &event.country {
            // Add your country shield rules here
            let blocked_countries = vec!["XX", "YY"]; // Example blocked countries
            if blocked_countries.contains(&country.as_str()) {
                return Err(EventServiceError::ShieldRuleBlocked("Blocked country".to_string()));
            }
        }
        Ok(())
    }

    // Helper function to safely get version string
    fn format_version(major: Option<&str>, minor: Option<&str>, patch: Option<&str>) -> String {
        format!("{}.{}.{}", 
            major.unwrap_or("0"),
            minor.unwrap_or("0"),
            patch.unwrap_or("0")
        )
    }

    // Helper function to safely insert into props
    fn insert_prop(obj: &mut serde_json::Map<String, serde_json::Value>, key: &str, value: impl Into<String>) {
        obj.insert(key.to_string(), serde_json::Value::String(value.into()));
    }

    fn put_user_agent(&self, event: &mut Event) -> Result<(), EventServiceError> {
        if let Some(ua_string) = &event.user_agent {
            if let Some(props) = &mut event.props {
                if let Some(obj) = props.as_object_mut() {
                    // Parse product (browser) information
                    let product = self.ua_parser.parse_product(ua_string);
                    Self::insert_prop(obj, "browser", product.name.unwrap_or(Cow::Borrowed("Unknown")).into_owned());
                    Self::insert_prop(obj, "browser_version", Self::format_version(
                        product.major.as_deref(),
                        product.minor.as_deref(),
                        product.patch.as_deref()
                    ));

                    // Parse OS information
                    let os = self.ua_parser.parse_os(ua_string);
                    Self::insert_prop(obj, "os", os.name.unwrap_or(Cow::Borrowed("Unknown")).into_owned());
                    Self::insert_prop(obj, "os_version", Self::format_version(
                        os.major.as_deref(),
                        os.minor.as_deref(),
                        os.patch.as_deref()
                    ));

                    // Parse device information
                    let device = self.ua_parser.parse_device(ua_string);
                    Self::insert_prop(obj, "device", device.name.unwrap_or(Cow::from("Unknown")).into_owned());
                    Self::insert_prop(obj, "device_brand", device.brand.unwrap_or(Cow::from("Unknown")).into_owned());
                    Self::insert_prop(obj, "device_model", device.model.unwrap_or(Cow::from("Unknown")).into_owned());

                    // Parse engine information
                    let engine = self.ua_parser.parse_engine(ua_string);
                    Self::insert_prop(obj, "engine", engine.name.unwrap_or(Cow::from("Unknown")).into_owned());
                    Self::insert_prop(obj, "engine_version", Self::format_version(
                        engine.major.as_deref(),
                        engine.minor.as_deref(),
                        engine.patch.as_deref()
                    ));

                    // Parse CPU information
                    let cpu = self.ua_parser.parse_cpu(ua_string);
                    if let Some(arch) = cpu.architecture {
                        Self::insert_prop(obj, "cpu_architecture", arch.into_owned());
                    }
                }
            }
        }
        Ok(())
    }

    fn put_referrer(&self, event: &mut Event) -> Result<(), EventServiceError> {
        if let Some(referrer) = &event.referrer {
            if let Ok(url) = Url::parse(referrer) {
                if let Some(props) = &mut event.props {
                    if let Some(obj) = props.as_object_mut() {
                        Self::insert_prop(obj, "referrer_domain", 
                            url.host_str().unwrap_or_default().to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn put_utm_tags(&self, event: &mut Event) -> Result<(), EventServiceError> {
        if let Ok(url) = Url::parse(&event.url) {
            let query_pairs: Vec<(String, String)> = url.query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .filter(|(k, _)| k.starts_with("utm_"))
                .collect();

            for (key, value) in query_pairs {
                match key.as_str() {
                    "utm_source" => event.utm_source = Some(value),
                    "utm_medium" => event.utm_medium = Some(value),
                    "utm_campaign" => event.utm_campaign = Some(value),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn register_session(&self, event: &mut Event) -> Result<(), EventServiceError> {
        // Generate or update session ID
        if event.session_id.is_none() {
            event.session_id = Some(uuid::Uuid::new_v4());
        }
        Ok(())
    }

    async fn buffer_event(&self, event: Event) -> Result<(), EventServiceError> {
        let mut buffer = self.buffer.lock().await;
        if buffer.len() >= self.config.max_buffer_size {
            return Err(EventServiceError::BufferFull);
        }
        buffer.push_back(event);
        Ok(())
    }

    pub async fn flush_buffer(&self) -> Result<(), EventServiceError> {
        let mut buffer = self.buffer.lock().await;
        if buffer.is_empty() {
            return Ok(());
        }

        let events: Vec<Event> = buffer.drain(..).collect();
        let mut client = self.pool.get_handle()
            .await
            .map_err(|e| EventServiceError::DatabaseError(e.to_string()))?;

        // Prepare batch insert query
        let query = "INSERT INTO events (id, name, url, domain, timestamp, user_id, session_id, \
                    referrer, user_agent, ip_address, country, city, props, revenue, \
                    utm_source, utm_medium, utm_campaign) VALUES";

        // TODO: Implement proper batch insert with proper value encoding
        // This is a placeholder implementation
        for event in events {
            client.execute(query)
                .await
                .map_err(|e| EventServiceError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    // Helper function to check URL parsing and get host
    fn parse_url_host(url_str: &str) -> Option<String> {
        Url::parse(url_str)
            .ok()
            .and_then(|url| url.host_str())
            .map(|host| host.to_string())
    }

    fn drop_spam_referrer(&self, event: &Event) -> Result<(), EventServiceError> {
        if let Some(referrer) = &event.referrer {
            if let Some(host) = Self::parse_url_host(referrer) {
                // Check if domain is in spam list
                if self.spam_rules.is_spam_domain(&host) {
                    return Err(EventServiceError::ShieldRuleBlocked(
                        format!("Spam referrer detected: {}", host)
                    ));
                }

                // Check for spam patterns
                if self.spam_rules.contains_spam_pattern(referrer) {
                    return Err(EventServiceError::ShieldRuleBlocked(
                        format!("Suspicious referrer pattern detected in: {}", host)
                    ));
                }
            }
        }
        Ok(())
    }
} 