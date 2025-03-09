use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ShieldRules {
    pub protected_paths: Vec<String>,
    pub bot_user_agents: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SpamRulesConfig {
    pub spam_domains: Vec<String>,
    pub spam_patterns: Vec<String>,
    pub shield_rules: ShieldRules,
}

impl SpamRulesConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .build()?;

        settings.try_deserialize()
    }

    pub fn is_spam_domain(&self, domain: &str) -> bool {
        self.spam_domains.iter().any(|spam_domain| {
            domain.ends_with(spam_domain)
        })
    }

    pub fn contains_spam_pattern(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        self.spam_patterns.iter().any(|pattern| {
            url_lower.contains(&pattern.to_lowercase())
        })
    }

    pub fn is_protected_path(&self, path: &str) -> bool {
        self.shield_rules.protected_paths.iter().any(|protected| {
            path.starts_with(protected)
        })
    }

    pub fn is_bot_user_agent(&self, user_agent: &str) -> bool {
        let ua_lower = user_agent.to_lowercase();
        self.shield_rules.bot_user_agents.iter().any(|bot| {
            ua_lower.contains(&bot.to_lowercase())
        })
    }
} 