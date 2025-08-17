use tyl_tracing::{Environment, SimpleTracer, TraceConfig, TracingManager};
use tyl_errors::TylError;

#[test]
fn test_end_to_end_tracing() {
    let config = TraceConfig::new("integration-test-service");
    let tracer = SimpleTracer::new(config);

    let span_id = tracer.start_span("end_to_end_operation", None).unwrap();
    tracer
        .add_span_attribute(&span_id, "test_id", serde_json::json!("integration_001"))
        .unwrap();
    tracer
        .add_span_attribute(&span_id, "environment", serde_json::json!("test"))
        .unwrap();
    tracer.end_span(span_id).unwrap();

    let completed_spans = tracer.get_completed_spans();
    assert_eq!(completed_spans.len(), 1);
    assert_eq!(completed_spans[0].operation_name, "end_to_end_operation");
    assert_eq!(completed_spans[0].attributes.len(), 2);
}

#[test]
fn test_configuration_integration() {
    let config = TraceConfig::new("config-test-service")
        .with_environment(Environment::Testing)
        .with_sampling_rate(0.75)
        .with_max_spans(50);

    let tracer = SimpleTracer::new(config);

    assert_eq!(tracer.config().service_name, "config-test-service");
    assert_eq!(tracer.config().environment, Environment::Testing);
    assert_eq!(tracer.config().sampling_rate, 0.75);
    assert_eq!(tracer.config().max_spans, 50);
}

#[test]
fn test_span_lifecycle_integration() {
    let tracer = SimpleTracer::new(TraceConfig::new("lifecycle-test"));

    // Start multiple spans
    let span1_id = tracer.start_span("operation_1", None).unwrap();
    let span2_id = tracer.start_span("operation_2", None).unwrap();
    let span3_id = tracer
        .start_span("operation_3", Some(span1_id.clone()))
        .unwrap();

    // Add attributes
    tracer
        .add_span_attribute(&span1_id, "type", serde_json::json!("parent"))
        .unwrap();
    tracer
        .add_span_attribute(&span3_id, "type", serde_json::json!("child"))
        .unwrap();

    // End spans in different order
    tracer.end_span(span2_id).unwrap();
    tracer.end_span(span3_id).unwrap();
    tracer.end_span(span1_id).unwrap();

    let completed_spans = tracer.get_completed_spans();
    assert_eq!(completed_spans.len(), 3);

    // Verify parent-child relationship
    let child_span = completed_spans
        .iter()
        .find(|s| s.operation_name == "operation_3")
        .unwrap();
    assert!(child_span.parent_span_id.is_some());
}

#[test]
fn test_baggage_context_integration() {
    let tracer = SimpleTracer::new(TraceConfig::new("baggage-test"));

    // Set baggage context
    tracer.set_baggage("request_id", "req_integration_123");
    tracer.set_baggage("user_session", "session_abc");
    tracer.set_baggage("trace_context", "ctx_xyz");

    // Create spans that use baggage
    let span_id = tracer.start_span("business_operation", None).unwrap();

    // Simulate using baggage in span
    if let Some(req_id) = tracer.get_baggage("request_id") {
        tracer
            .add_span_attribute(&span_id, "request_id", serde_json::json!(req_id))
            .unwrap();
    }

    if let Some(session) = tracer.get_baggage("user_session") {
        tracer
            .add_span_attribute(&span_id, "session", serde_json::json!(session))
            .unwrap();
    }

    tracer.end_span(span_id).unwrap();

    // Verify baggage retrieval
    assert_eq!(
        tracer.get_baggage("request_id"),
        Some("req_integration_123".to_string())
    );
    assert_eq!(
        tracer.get_baggage("user_session"),
        Some("session_abc".to_string())
    );
    assert_eq!(tracer.get_baggage("non_existent"), None);

    // Verify span has baggage-derived attributes
    let completed_spans = tracer.get_completed_spans();
    let span = &completed_spans[0];
    assert_eq!(
        span.attributes["request_id"],
        serde_json::json!("req_integration_123")
    );
    assert_eq!(span.attributes["session"], serde_json::json!("session_abc"));
}

#[test]
fn test_error_handling_integration() {
    let tracer = SimpleTracer::new(TraceConfig::new("error-test"));

    // Test invalid span operations
    let result = tracer.end_span("non_existent_span".to_string());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TylError::Validation { .. }
    ));

    let result = tracer.add_span_attribute("non_existent_span", "key", serde_json::json!("value"));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TylError::Validation { .. }
    ));

    // Test that valid operations still work after errors
    let span_id = tracer.start_span("valid_operation", None).unwrap();
    tracer
        .add_span_attribute(&span_id, "test", serde_json::json!("valid"))
        .unwrap();
    tracer.end_span(span_id).unwrap();

    assert_eq!(tracer.get_completed_spans().len(), 1);
}

#[test]
fn test_max_spans_boundary_integration() {
    let config = TraceConfig::new("boundary-test").with_max_spans(3);
    let tracer = SimpleTracer::new(config);

    // Create more spans than the limit
    for i in 0..5 {
        let span_id = tracer
            .start_span(&format!("operation_{}", i), None)
            .unwrap();
        tracer
            .add_span_attribute(&span_id, "iteration", serde_json::json!(i))
            .unwrap();
        tracer.end_span(span_id).unwrap();
    }

    let completed_spans = tracer.get_completed_spans();

    // Should respect the max_spans limit
    assert_eq!(completed_spans.len(), 3);

    // Should keep the most recent spans
    assert_eq!(completed_spans[0].operation_name, "operation_2");
    assert_eq!(completed_spans[1].operation_name, "operation_3");
    assert_eq!(completed_spans[2].operation_name, "operation_4");
}

#[test]
fn test_concurrent_access_safety() {
    use std::sync::Arc;
    use std::thread;

    let tracer = Arc::new(SimpleTracer::new(TraceConfig::new("concurrent-test")));
    let mut handles = vec![];

    // Spawn multiple threads that create and end spans
    for i in 0..5 {
        let tracer_clone = tracer.clone();
        let handle = thread::spawn(move || {
            let span_id = tracer_clone
                .start_span(&format!("thread_operation_{}", i), None)
                .unwrap();
            tracer_clone
                .add_span_attribute(&span_id, "thread_id", serde_json::json!(i))
                .unwrap();

            // Set some baggage
            tracer_clone.set_baggage(&format!("thread_{}_key", i), &format!("thread_{}_value", i));

            tracer_clone.end_span(span_id).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all spans were created and completed
    let completed_spans = tracer.get_completed_spans();
    assert_eq!(completed_spans.len(), 5);

    // Verify baggage from all threads
    for i in 0..5 {
        let key = format!("thread_{}_key", i);
        let expected_value = format!("thread_{}_value", i);
        assert_eq!(tracer.get_baggage(&key), Some(expected_value));
    }
}
