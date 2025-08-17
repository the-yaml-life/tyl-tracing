// Test script to verify the public API is preserved after refactoring
use tyl_tracing::{
    TracingManager, SimpleTracer, TraceConfig, Environment, 
    Span, SpanStatus, generate_span_id, generate_trace_id,
    TracingResult
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test TraceConfig creation and builder pattern
    let config = TraceConfig::new("test-service")
        .with_environment(Environment::Development)
        .with_sampling_rate(0.5)
        .with_max_spans(100);
    
    // Test SimpleTracer creation
    let tracer = SimpleTracer::new(config);
    
    // Test TracingManager trait methods
    let span_id = tracer.start_span("test_operation", None)?;
    tracer.add_span_attribute(&span_id, "test_key", serde_json::json!("test_value"))?;
    tracer.set_baggage("request_id", "req123");
    let baggage = tracer.get_baggage("request_id");
    tracer.end_span(span_id)?;
    
    let completed_spans = tracer.get_completed_spans();
    
    // Test utility functions
    let _span_id = generate_span_id();
    let _trace_id = generate_trace_id();
    
    // Test Environment
    let _env = Environment::from_env();
    
    println!("All API functions are accessible!");
    println!("Completed spans: {}", completed_spans.len());
    println!("Baggage retrieved: {:?}", baggage);
    
    Ok(())
}