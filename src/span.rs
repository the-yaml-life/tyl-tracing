//! Span management module
//!
//! Contains the Span struct, SpanStatus enum, and related functionality for
//! managing distributed tracing spans.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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
