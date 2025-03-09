# Analytics API

A high-performance analytics API built with Rust and Rocket, designed for event tracking and analytics.

## Features

- Event ingestion with buffering
- Geolocation tracking
- User agent parsing
- UTM parameter extraction
- Rate limiting
- ClickHouse integration for efficient storage
- Asynchronous processing

## Prerequisites

- Rust 1.70 or later
- ClickHouse database
- GeoIP2 database (optional)

## Setup

1. Clone the repository
2. Create a `.env` file with the following variables:
   ```
   DATABASE_URL=clickhouse://user:password@localhost:9000/analytics
   MAX_BUFFER_SIZE=10000
   FLUSH_INTERVAL_SECONDS=10
   GEOIP_DB_PATH=/path/to/GeoIP2-City.mmdb
   RATE_LIMIT_PER_MINUTE=60
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

4. Run the server:
   ```bash
   cargo run --release
   ```

## API Endpoints

### POST /api/event

Ingest a new analytics event.

**Request Body:**
```json
{
  "name": "pageview",
  "url": "https://example.com",
  "domain": "example.com",
  "props": {
    "button_id": "signup",
    "button_text": "Sign Up"
  },
  "user_id": "optional-user-id",
  "revenue": 99.99
}
```

**Response Codes:**
- 202 Accepted: Event processed successfully
- 400 Bad Request: Invalid request parameters
- 429 Too Many Requests: Rate limit exceeded
- 500 Internal Server Error: Server error

## Architecture

The API follows a pipeline architecture for event processing:

1. Request Validation
2. Event Enrichment
   - Geolocation
   - User Agent Parsing
   - UTM Parameter Extraction
3. Event Buffering
4. Batch Processing to ClickHouse

## Performance Considerations

- Events are buffered in memory before being written to ClickHouse
- Batch processing reduces database load
- Rate limiting prevents overload
- Asynchronous processing for better throughput

## Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a new Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details. 