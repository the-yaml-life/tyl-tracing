use tyl_tracing::{Environment, SimpleTracer, TraceConfig, TracingManager};

fn main() -> Result<(), tyl_tracing::TracingError> {
    println!("=== TYL Tracing Basic Usage ===\n");

    // Basic usage example
    basic_usage_example()?;

    // Custom configuration example
    custom_config_example()?;

    // Span hierarchy example
    span_hierarchy_example()?;

    // Baggage example
    baggage_example()?;

    Ok(())
}

fn basic_usage_example() -> Result<(), tyl_tracing::TracingError> {
    println!("--- Basic Usage ---");

    let config = TraceConfig::new("example-service");
    let tracer = SimpleTracer::new(config);

    let span_id = tracer.start_span("user_login", None)?;
    tracer.add_span_attribute(&span_id, "user_id", serde_json::json!("user123"))?;
    tracer.add_span_attribute(&span_id, "ip_address", serde_json::json!("192.168.1.1"))?;
    tracer.end_span(span_id)?;

    let completed_spans = tracer.get_completed_spans();
    println!("✅ Completed {} spans", completed_spans.len());

    if let Some(span) = completed_spans.first() {
        println!("  - Operation: {}", span.operation_name);
        println!("  - Duration: {:?}ms", span.duration_ms());
        println!("  - Attributes: {:?}", span.attributes);
    }

    println!();
    Ok(())
}

fn custom_config_example() -> Result<(), tyl_tracing::TracingError> {
    println!("--- Custom Configuration ---");

    let config = TraceConfig::new("custom-service")
        .with_environment(Environment::Production)
        .with_sampling_rate(0.8)
        .with_max_spans(100);

    let tracer = SimpleTracer::new(config);

    println!("✅ Service: {}", tracer.config().service_name);
    println!("  - Environment: {:?}", tracer.config().environment);
    println!("  - Sampling Rate: {}", tracer.config().sampling_rate);
    println!("  - Max Spans: {}", tracer.config().max_spans);

    // Create multiple spans to test config
    for i in 0..5 {
        let span_id = tracer.start_span(&format!("operation_{}", i), None)?;
        tracer.add_span_attribute(&span_id, "iteration", serde_json::json!(i))?;
        tracer.end_span(span_id)?;
    }

    println!(
        "  - Completed spans: {}",
        tracer.get_completed_spans().len()
    );
    println!();
    Ok(())
}

fn span_hierarchy_example() -> Result<(), tyl_tracing::TracingError> {
    println!("--- Span Hierarchy ---");

    let tracer = SimpleTracer::new(TraceConfig::new("hierarchy-service"));

    // Parent span
    let parent_span_id = tracer.start_span("http_request", None)?;
    tracer.add_span_attribute(&parent_span_id, "method", serde_json::json!("POST"))?;
    tracer.add_span_attribute(&parent_span_id, "url", serde_json::json!("/api/users"))?;

    // Child spans
    let db_span_id = tracer.start_span("database_query", Some(parent_span_id.clone()))?;
    tracer.add_span_attribute(
        &db_span_id,
        "query",
        serde_json::json!("SELECT * FROM users"),
    )?;
    tracer.end_span(db_span_id)?;

    let validation_span_id = tracer.start_span("input_validation", Some(parent_span_id.clone()))?;
    tracer.add_span_attribute(
        &validation_span_id,
        "fields",
        serde_json::json!(["email", "name"]),
    )?;
    tracer.end_span(validation_span_id)?;

    tracer.end_span(parent_span_id)?;

    let completed_spans = tracer.get_completed_spans();
    println!(
        "✅ Created span hierarchy with {} spans:",
        completed_spans.len()
    );

    for span in &completed_spans {
        let indent = if span.parent_span_id.is_some() {
            "  └─ "
        } else {
            "─ "
        };
        println!(
            "{}[{}] {} ({}ms)",
            indent,
            &span.span_id[..8],
            span.operation_name,
            span.duration_ms().unwrap_or(0)
        );
    }

    println!();
    Ok(())
}

fn baggage_example() -> Result<(), tyl_tracing::TracingError> {
    println!("--- Baggage (Context Propagation) ---");

    let tracer = SimpleTracer::new(TraceConfig::new("baggage-service"));

    // Set baggage for trace context
    tracer.set_baggage("request_id", "req_12345");
    tracer.set_baggage("user_id", "user_67890");
    tracer.set_baggage("correlation_id", "corr_abcdef");

    println!("✅ Baggage set for trace context:");
    println!("  - Request ID: {:?}", tracer.get_baggage("request_id"));
    println!("  - User ID: {:?}", tracer.get_baggage("user_id"));
    println!(
        "  - Correlation ID: {:?}",
        tracer.get_baggage("correlation_id")
    );
    println!("  - Non-existent: {:?}", tracer.get_baggage("non_existent"));

    // Create spans that can use the baggage
    let span_id = tracer.start_span("business_logic", None)?;

    // In a real implementation, baggage would be automatically
    // propagated to child spans and across service boundaries
    if let Some(req_id) = tracer.get_baggage("request_id") {
        tracer.add_span_attribute(&span_id, "request_id", serde_json::json!(req_id))?;
    }

    tracer.end_span(span_id)?;

    println!("  - Span created with baggage context");
    println!();
    Ok(())
}
