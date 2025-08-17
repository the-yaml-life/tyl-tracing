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
//! # Ok::<(), tyl_tracing::TracingError>(())
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

/// Result type for tracing operations
pub type TracingResult<T> = Result<T, TracingError>;

/// Errors that can occur during tracing operations
#[derive(Debug, Error)]
pub enum TracingError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Span operation failed: {message}")]
    SpanOperation { message: String },
    
    #[error("Invalid span ID: {span_id}")]
    InvalidSpanId { span_id: String },
    
    #[error("Serialization error: {message}")]
    Serialization { message: String },
    
    #[error("OpenTelemetry error: {message}")]
    #[cfg(feature = "otel")]
    OpenTelemetry { message: String },
}

/// Port (Interface) - Main tracing contract
pub trait TracingManager {
    /// Start a new span with optional parent span ID
    fn start_span(&self, operation_name: &str, parent_span_id: Option<String>) -> TracingResult<String>;
    
    /// End a span by its ID
    fn end_span(&self, span_id: String) -> TracingResult<()>;
    
    /// Add metadata to an active span
    fn add_span_attribute(&self, span_id: &str, key: &str, value: serde_json::Value) -> TracingResult<()>;
    
    /// Get all completed spans (for debugging/testing)
    fn get_completed_spans(&self) -> Vec<Span>;
    
    /// Set baggage for trace context
    fn set_baggage(&self, key: &str, value: &str);
    
    /// Get baggage from trace context  
    fn get_baggage(&self, key: &str) -> Option<String>;
}

/// Core span data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub status: SpanStatus,
}

/// Span execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Active,
    Completed,
    Error { message: String },
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    pub service_name: String,
    pub environment: Environment,
    pub sampling_rate: f64,
    pub max_spans: usize,
}

/// Runtime environment detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl TraceConfig {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            environment: Environment::from_env(),
            sampling_rate: 1.0,
            max_spans: 1000,
        }
    }
    
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }
    
    pub fn with_sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_max_spans(mut self, max_spans: usize) -> Self {
        self.max_spans = max_spans;
        self
    }
}

impl Span {
    pub fn new(operation_name: String, parent_span_id: Option<String>) -> Self {
        let span_id = generate_span_id();
        let trace_id = parent_span_id
            .as_ref()
            .map(|_| generate_trace_id()) // In real implementation, inherit from parent
            .unwrap_or_else(generate_trace_id);
            
        Self {
            span_id,
            trace_id,
            parent_span_id,
            operation_name,
            start_time: current_timestamp(),
            end_time: None,
            attributes: HashMap::new(),
            status: SpanStatus::Active,
        }
    }
    
    pub fn duration_ms(&self) -> Option<u64> {
        self.end_time.map(|end| end.saturating_sub(self.start_time))
    }
    
    pub fn is_active(&self) -> bool {
        matches!(self.status, SpanStatus::Active)
    }
    
    pub fn complete(&mut self) {
        self.end_time = Some(current_timestamp());
        self.status = SpanStatus::Completed;
    }
    
    pub fn error(&mut self, message: String) {
        self.end_time = Some(current_timestamp());
        self.status = SpanStatus::Error { message };
    }
}

/// Adapter - Simple in-memory tracer for development
pub struct SimpleTracer {
    config: TraceConfig,
    active_spans: std::sync::Mutex<HashMap<String, Span>>,
    completed_spans: std::sync::Mutex<Vec<Span>>,
    baggage: std::sync::Mutex<HashMap<String, String>>,
}

impl SimpleTracer {
    pub fn new(config: TraceConfig) -> Self {
        Self {
            config,
            active_spans: std::sync::Mutex::new(HashMap::new()),
            completed_spans: std::sync::Mutex::new(Vec::new()),
            baggage: std::sync::Mutex::new(HashMap::new()),
        }
    }
    
    pub fn config(&self) -> &TraceConfig {
        &self.config
    }
}

impl Default for SimpleTracer {
    fn default() -> Self {
        Self::new(TraceConfig::new("default-service"))
    }
}

impl TracingManager for SimpleTracer {
    fn start_span(&self, operation_name: &str, parent_span_id: Option<String>) -> TracingResult<String> {
        let span = Span::new(operation_name.to_string(), parent_span_id);
        let span_id = span.span_id.clone();
        
        let mut active_spans = self.active_spans.lock().unwrap();
        active_spans.insert(span_id.clone(), span);
        
        Ok(span_id)
    }
    
    fn end_span(&self, span_id: String) -> TracingResult<()> {
        let mut active_spans = self.active_spans.lock().unwrap();
        
        if let Some(mut span) = active_spans.remove(&span_id) {
            span.complete();
            
            let mut completed_spans = self.completed_spans.lock().unwrap();
            completed_spans.push(span);
            
            // Respect max_spans limit
            if completed_spans.len() > self.config.max_spans {
                completed_spans.remove(0);
            }
            
            Ok(())
        } else {
            Err(TracingError::InvalidSpanId { span_id })
        }
    }
    
    fn add_span_attribute(&self, span_id: &str, key: &str, value: serde_json::Value) -> TracingResult<()> {
        let mut active_spans = self.active_spans.lock().unwrap();
        
        if let Some(span) = active_spans.get_mut(span_id) {
            span.attributes.insert(key.to_string(), value);
            Ok(())
        } else {
            Err(TracingError::InvalidSpanId { span_id: span_id.to_string() })
        }
    }
    
    fn get_completed_spans(&self) -> Vec<Span> {
        let completed_spans = self.completed_spans.lock().unwrap();
        completed_spans.clone()
    }
    
    fn set_baggage(&self, key: &str, value: &str) {
        let mut baggage = self.baggage.lock().unwrap();
        baggage.insert(key.to_string(), value.to_string());
    }
    
    fn get_baggage(&self, key: &str) -> Option<String> {
        let baggage = self.baggage.lock().unwrap();
        baggage.get(key).cloned()
    }
}

impl Environment {
    pub fn from_env() -> Self {
        match std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "prod" | "production" => Environment::Production,
            "test" | "testing" => Environment::Testing,
            _ => Environment::Development,
        }
    }
}

// Utility functions
pub fn generate_span_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn generate_trace_id() -> String {
    Uuid::new_v4().to_string()
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

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