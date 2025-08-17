//! # TYL Tracing
//!
//! Distributed tracing library for TYL framework with OpenTelemetry support and hexagonal architecture.
//!
//! ## Features
//!
//! - Simple in-memory tracing for development
//! - OpenTelemetry integration for production (optional)
//! - Hexagonal architecture with ports and adapters
//! - Span correlation and context propagation
//! - Multiple output formats (JSON, pretty-print)
//! - Async/await support
//!
//! ## Quick Start
//!
//! ```rust
//! use tyl_tracing::{TracingManager, SimpleTracer, TraceConfig};
//!
//! // Basic usage
//! let config = TraceConfig::new("my-service");
//! let tracer = SimpleTracer::new(config);
//! 
//! let span_id = tracer.start_span("user_operation", None)?;
//! // ... do work ...
//! tracer.end_span(span_id)?;
//! # Ok::<(), tyl_errors::TylError>(())
//! ```
//!
//! ## Architecture
//!
//! This module follows hexagonal architecture:
//!
//! - **Port (Interface)**: `TracingManager` - defines the tracing contract
//! - **Adapters**: 
//!   - `SimpleTracer` - In-memory tracing for development
//!   - `OpenTelemetryTracer` - Production tracing with OTLP (optional)
//! - **Domain Logic**: Span management and correlation
//!
//! ## Examples
//!
//! See the `examples/` directory for complete usage examples.

// Module declarations
pub mod config;
pub mod span;
pub mod tracer;

// Re-exports for public API
pub use config::{TraceConfig, Environment};
pub use span::{Span, SpanStatus, generate_span_id, generate_trace_id};
pub use tracer::{TracingManager, SimpleTracer, TracingResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new("test_operation".to_string(), None);
        assert_eq!(span.operation_name, "test_operation");
        assert!(span.is_active());
        assert!(span.end_time.is_none());
    }

    #[test]
    fn test_span_completion() {
        let mut span = Span::new("test_operation".to_string(), None);
        span.complete();
        assert!(!span.is_active());
        assert!(span.end_time.is_some());
        assert!(matches!(span.status, SpanStatus::Completed));
    }

    #[test]
    fn test_simple_tracer_basic_functionality() {
        let config = TraceConfig::new("test-service");
        let tracer = SimpleTracer::new(config);
        
        let span_id = tracer.start_span("user_action", None).unwrap();
        assert!(!span_id.is_empty());
        
        tracer.add_span_attribute(&span_id, "user_id", serde_json::json!("user123")).unwrap();
        tracer.end_span(span_id).unwrap();
        
        let completed_spans = tracer.get_completed_spans();
        assert_eq!(completed_spans.len(), 1);
        assert_eq!(completed_spans[0].operation_name, "user_action");
    }

    #[test]
    fn test_trace_config_builder() {
        let config = TraceConfig::new("test-service")
            .with_environment(Environment::Production)
            .with_sampling_rate(0.5)
            .with_max_spans(500);
            
        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.environment, Environment::Production);
        assert_eq!(config.sampling_rate, 0.5);
        assert_eq!(config.max_spans, 500);
    }

    #[test]
    fn test_baggage_operations() {
        let tracer = SimpleTracer::default();
        
        tracer.set_baggage("request_id", "req123");
        tracer.set_baggage("user_id", "user456");
        
        assert_eq!(tracer.get_baggage("request_id"), Some("req123".to_string()));
        assert_eq!(tracer.get_baggage("user_id"), Some("user456".to_string()));
        assert_eq!(tracer.get_baggage("nonexistent"), None);
    }

    #[test]
    fn test_invalid_span_operations() {
        let tracer = SimpleTracer::default();
        
        let result = tracer.end_span("invalid_span_id".to_string());
        assert!(result.is_err());
        
        let result = tracer.add_span_attribute("invalid_span_id", "key", serde_json::json!("value"));
        assert!(result.is_err());
    }

    #[test]
    fn test_span_hierarchy() {
        let tracer = SimpleTracer::default();
        
        let parent_span_id = tracer.start_span("parent_operation", None).unwrap();
        let child_span_id = tracer.start_span("child_operation", Some(parent_span_id.clone())).unwrap();
        
        tracer.end_span(child_span_id).unwrap();
        tracer.end_span(parent_span_id).unwrap();
        
        let completed_spans = tracer.get_completed_spans();
        assert_eq!(completed_spans.len(), 2);
    }

    #[test]
    fn test_max_spans_limit() {
        let config = TraceConfig::new("test-service").with_max_spans(2);
        let tracer = SimpleTracer::new(config);
        
        // Create 3 spans, should only keep 2
        for i in 0..3 {
            let span_id = tracer.start_span(&format!("operation_{}", i), None).unwrap();
            tracer.end_span(span_id).unwrap();
        }
        
        let completed_spans = tracer.get_completed_spans();
        assert_eq!(completed_spans.len(), 2);
        // Should keep the last 2 spans
        assert_eq!(completed_spans[0].operation_name, "operation_1");
        assert_eq!(completed_spans[1].operation_name, "operation_2");
    }

    #[test]
    fn test_environment_detection() {
        let env = Environment::from_env();
        assert!(matches!(env, Environment::Development | Environment::Production | Environment::Testing));
    }
}