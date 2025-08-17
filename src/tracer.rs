//! Tracing management module
//!
//! Contains the TracingManager trait (port) and SimpleTracer implementation (adapter)
//! following hexagonal architecture principles.

use std::collections::HashMap;
use tyl_errors::{TylError, TylResult};
use crate::config::TraceConfig;
use crate::span::Span;

/// Result type for tracing operations using unified TYL error handling
pub type TracingResult<T> = TylResult<T>;

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
            Err(TylError::validation("span_id", format!("invalid span ID: {}", span_id)))
        }
    }
    
    fn add_span_attribute(&self, span_id: &str, key: &str, value: serde_json::Value) -> TracingResult<()> {
        let mut active_spans = self.active_spans.lock().unwrap();
        
        if let Some(span) = active_spans.get_mut(span_id) {
            span.attributes.insert(key.to_string(), value);
            Ok(())
        } else {
            Err(TylError::validation("span_id", format!("invalid span ID: {}", span_id)))
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