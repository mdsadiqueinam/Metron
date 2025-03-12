# Rust Analytics: Google Analytics Alternative

## Project Overview

This document outlines the architecture and features for a Google Analytics alternative built in Rust. The platform will provide web analytics with multi-tenant support, allowing different companies to use the service with their own subdomain.

## Technology Stack

- **Backend**: Rust with Axum web framework
- **Database**: PostgreSQL with SQLx
- **Frontend**: JavaScript tracker + Dashboard (TBD)
- **Infrastructure**: Multi-tenant architecture with subdomain routing

## Project Structure

```
analytics-rust/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── api/
│   ├── collector/
│   ├── models/
│   ├── storage/
│   └── processors/
└── js-tracker/
    └── tracker.js
```

## Core Components

### 1. Cargo Dependencies

```toml
[package]
name = "analytics-rust"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web server
axum = "0.6"
tower-http = { version = "0.4", features = ["cors"] }
tokio = { version = "1", features = ["full"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "chrono", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.3", features = ["v4", "serde"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Environment variables
dotenv = "0.15"
```

### 2. Main Application Structure

```rust
use axum::{
    routing::{get, post},
    Router, Server,
};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod collector;
mod models;
mod processors;
mod storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Setup database connection
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Build our application with routes
    let app = Router::new()
        // Data collection endpoint
        .route("/collect", post(collector::collect_data))
        // API endpoints
        .route("/api/stats", get(api::get_stats))
        // Attach CORS middleware
        .layer(cors)
        // Add database connection to all routes
        .with_state(pool);
    
    // Run the server
    let addr = env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    tracing::info!("Starting server on {}", addr);
    
    Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

### 3. Data Models

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PageView {
    pub id: Option<Uuid>,
    pub site_id: String,
    pub url: String,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub screen_width: Option<i32>,
    pub screen_height: Option<i32>,
    pub timestamp: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
    pub country: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<Uuid>,
    pub site_id: String,
    pub name: String,
    pub url: String,
    pub session_id: Option<String>,
    pub properties: Option<serde_json::Value>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub created_at: DateTime<Utc>,
    pub company_id: String, // Link to company for multi-tenancy
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Company {
    pub id: String,
    pub name: String,
    pub subdomain: String,
    pub created_at: DateTime<Utc>,
    pub settings: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub company_id: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stats {
    pub visitors: i64,
    pub pageviews: i64,
    pub bounce_rate: f64,
    pub avg_session_duration: f64,
}
```

### 4. JavaScript Tracker

```javascript
// Simple analytics tracker
(function() {
  const ANALYTICS_ENDPOINT = 'https://your-analytics-domain.com/collect';
  const SITE_ID = window.ANALYTICS_SITE_ID || document.currentScript.getAttribute('data-site-id');
  
  if (!SITE_ID) {
    console.error('Analytics: No site ID provided');
    return;
  }
  
  // Generate a session ID if none exists
  let sessionId = localStorage.getItem('analytics_session_id');
  if (!sessionId) {
    sessionId = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
      const r = Math.random() * 16 | 0, v = c == 'x' ? r : (r & 0x3 | 0x8);
      return v.toString(16);
    });
    localStorage.setItem('analytics_session_id', sessionId);
    localStorage.setItem('analytics_session_start', Date.now());
  }
  
  // Send pageview
  function sendPageview() {
    const data = {
      type: 'pageview',
      site_id: SITE_ID,
      url: window.location.href,
      referrer: document.referrer || null,
      screen_width: window.innerWidth,
      screen_height: window.innerHeight,
      session_id: sessionId
    };
    
    fetch(ANALYTICS_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(data),
      // Keep-alive can improve performance for frequent events
      keepalive: true
    }).catch(err => console.error('Analytics error:', err));
  }
  
  // Track events
  window.trackEvent = function(name, properties = {}) {
    const data = {
      type: 'event',
      site_id: SITE_ID,
      name: name,
      url: window.location.href,
      session_id: sessionId,
      properties: properties
    };
    
    fetch(ANALYTICS_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(data),
      keepalive: true
    }).catch(err => console.error('Analytics error:', err));
  };
  
  // Send pageview on page load
  sendPageview();
  
  // Send pageview on route change for SPAs
  let lastUrl = window.location.href;
  setInterval(() => {
    if (window.location.href !== lastUrl) {
      lastUrl = window.location.href;
      sendPageview();
    }
  }, 500);
})();
```

### 5. Database Schema

```sql
-- Companies table for multi-tenant support
CREATE TABLE companies (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    subdomain VARCHAR(100) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settings JSONB NOT NULL DEFAULT '{}'
);

-- Users table for authentication
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    company_id VARCHAR(50) REFERENCES companies(id),
    role VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Sites table
CREATE TABLE sites (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    domain VARCHAR(255) NOT NULL,
    company_id VARCHAR(50) NOT NULL REFERENCES companies(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Pageviews table
CREATE TABLE pageviews (
    id UUID PRIMARY KEY,
    site_id VARCHAR(50) NOT NULL REFERENCES sites(id),
    url TEXT NOT NULL,
    referrer TEXT,
    user_agent TEXT,
    ip_address VARCHAR(45),
    screen_width INTEGER,
    screen_height INTEGER,
    timestamp TIMESTAMPTZ NOT NULL,
    session_id VARCHAR(50),
    country VARCHAR(2),
    browser VARCHAR(50),
    os VARCHAR(50),
    device_type VARCHAR(20)
);

-- Events table
CREATE TABLE events (
    id UUID PRIMARY KEY,
    site_id VARCHAR(50) NOT NULL REFERENCES sites(id),
    name VARCHAR(100) NOT NULL,
    url TEXT NOT NULL,
    session_id VARCHAR(50),
    properties JSONB,
    timestamp TIMESTAMPTZ NOT NULL
);

-- Indexes for performance
CREATE INDEX idx_pageviews_site_id ON pageviews(site_id);
CREATE INDEX idx_pageviews_timestamp ON pageviews(timestamp);
CREATE INDEX idx_pageviews_session_id ON pageviews(session_id);
CREATE INDEX idx_events_site_id ON events(site_id);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_name ON events(name);
CREATE INDEX idx_sites_company_id ON sites(company_id);
```

## Feature Set

### Core Analytics Features
1. **Page View Tracking** - Record visitors on each page
2. **Event Tracking** - Custom event collection (clicks, form submissions)
3. **Session Analytics** - Track user journeys within a visit
4. **Referrer Tracking** - Identify traffic sources
5. **Campaign Tracking** - UTM parameter support
6. **Custom Dimensions/Metrics** - Allow arbitrary data collection

### Data Collection
1. **JavaScript Tracker** - Lightweight client-side script
2. **Server-Side API** - Direct server-to-server tracking
3. **Batch Processing** - Handle high-volume collection
4. **Single Page App Support** - History API integration

### Data Enrichment
1. **Geographic Data** - Country/region detection
2. **Device Information** - Browser, OS, screen size
3. **Bot Filtering** - Exclude crawler traffic
4. **Performance Metrics** - Page load times, resource timing
5. **Bounce Rate Calculation** - Single-page visit detection

### Reporting
1. **Real-Time Dashboard** - Current active users
2. **Historical Reports** - Time-based comparisons
3. **Data Visualization** - Charts and graphs
4. **Export Capabilities** - CSV/JSON data export
5. **Top Content Analysis** - Most visited pages

### Technical Features
1. **High-Performance Data Collection** - Optimized ingestion
2. **Efficient Storage** - Time-series data management
3. **Horizontal Scaling** - Distributed processing
4. **Data Sampling** - Handle high-volume sites
5. **Data Retention Policies** - GDPR compliance

## Multi-Tenant Architecture

### Subdomain-Based Tenancy Features
1. **Subdomain-Based Tenancy** - Each company gets `{company-name}.analytics.com`
2. **Custom Subdomain Selection** - Companies choose their preferred subdomain
3. **Subdomain Validation** - Ensure uniqueness and proper formatting
4. **DNS Management** - Automated subdomain provisioning
5. **Wildcard SSL Support** - Secure all tenant subdomains

### Company Management Features
1. **Multi-Tenant Architecture** - Isolated data per company
2. **Company Registration** - Self-service signup with subdomain selection
3. **Workspace Customization** - Brand dashboards with company logo/colors
4. **Tenant Routing** - Route requests to correct tenant based on subdomain
5. **Company-Level Settings** - Global configurations per organization

### User & Access Control
1. **Team Management** - Add/remove users within a company
2. **Role-Based Access** - Admin, analyst, viewer permissions
3. **Single Sign-On** - SAML/OAuth integration with company IdPs
4. **Two-Factor Authentication** - Enhanced security option
5. **Audit Logs** - Track user actions within company account

### Data Isolation
1. **Tenant Data Separation** - Complete isolation between companies
2. **Multi-Site Management** - Track multiple properties per company
3. **Data Ownership Controls** - Company-specific data retention policies
4. **Access Scoping** - Limit user access to specific sites/properties
5. **Data Portability** - Allow companies to export their own data

### Subscription & Billing
1. **Tiered Pricing Plans** - Based on traffic volume/features
2. **Usage Monitoring** - Track company resource consumption
3. **Billing Management** - Payment processing and invoicing
4. **Feature Gating** - Control access to premium analytics features
5. **Subscription Administration** - Upgrade/downgrade workflows

## Why Axum over Rocket

For our analytics platform, we've chosen Axum (with Tower) instead of Rocket for the following reasons:

1. **Performance** - Axum is built on Tokio/Hyper and optimized for high throughput, critical for analytics collection
2. **Middleware system** - Tower's composable middleware architecture is ideal for analytics processing pipelines
3. **Scalability** - Better suited for services that need to handle many concurrent connections
4. **Lower overhead** - More lightweight, which matters for a high-volume endpoint
5. **Flexibility** - Tower middleware is highly composable for building custom data processing flows

## Implementation Roadmap

1. **Phase 1: Core Data Collection**
   - Basic server setup with Axum
   - JS tracker implementation
   - Database schema creation
   - Pageview and event collection endpoints

2. **Phase 2: Multi-Tenant Foundation**
   - Company registration and authentication
   - Subdomain routing system
   - User management and permissions

3. **Phase 3: Data Processing**
   - Session tracking
   - Bot detection
   - Geographic data enrichment
   - Performance metrics calculation

4. **Phase 4: Reporting Dashboard**
   - Real-time statistics
   - Historical data visualization
   - Report generation
   - Export functionality

5. **Phase 5: Advanced Features**
   - Custom dimensions/metrics
   - Funnel analysis
   - A/B test tracking
   - Conversion monitoring

6. **Phase 6: Enterprise Features**
   - SSO integration
   - Advanced access control
   - Data retention policies
   - Custom dashboards

## Next Steps

1. Set up the basic project structure
2. Implement core data models
3. Create the collection endpoint
4. Develop the JavaScript tracker
5. Create company and user authentication
6. Design the subdomain routing middleware
