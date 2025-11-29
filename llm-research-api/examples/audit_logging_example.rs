//! Example demonstrating the audit logging system
//!
//! Run with: cargo run --example audit_logging_example

use llm_research_api::security::*;
use llm_research_core::domain::UserId;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== Audit Logging System Examples ===\n");

    // Example 1: Basic audit logging with tracing writer
    println!("1. Basic Audit Logging");
    example_basic_logging().await?;

    // Example 2: File-based audit logging with rotation
    println!("\n2. File-Based Audit Logging");
    example_file_logging().await?;

    // Example 3: Composite writer (multiple destinations)
    println!("\n3. Composite Audit Writer");
    example_composite_logging().await?;

    // Example 4: Logging various types of events
    println!("\n4. Various Event Types");
    example_event_types().await?;

    // Example 5: Logging with detailed metadata
    println!("\n5. Detailed Metadata Logging");
    example_detailed_logging().await?;

    println!("\n=== Examples Complete ===");

    Ok(())
}

/// Example 1: Basic audit logging with tracing writer
async fn example_basic_logging() -> Result<(), Box<dyn std::error::Error>> {
    let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

    // Log successful authentication
    let actor = AuditActor::User {
        id: UserId::new(),
        email: "alice@example.com".to_string(),
    };
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

    logger.log_auth_success(&actor, ip).await?;
    println!("✓ Logged successful authentication");

    // Log failed authentication
    logger
        .log_auth_failure("bob@example.com", "Invalid password", ip)
        .await?;
    println!("✓ Logged failed authentication");

    Ok(())
}

/// Example 2: File-based audit logging with rotation
async fn example_file_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for the example
    let temp_dir = tempfile::TempDir::new()?;
    let log_path = temp_dir.path().join("audit.log");

    let writer = FileAuditWriter::new(log_path.clone())
        .with_max_size(10 * 1024) // 10 KB for demo
        .with_max_files(5);

    let logger = AuditLogger::new(Box::new(writer));

    // Log multiple events
    for i in 0..3 {
        let event = AuditEvent::new(
            AuditEventType::DataAccess,
            AuditActor::System,
            AuditResource::Experiment {
                id: Uuid::new_v4(),
            },
            AuditAction::Read,
            AuditOutcome::Success,
        )
        .with_details(json!({ "iteration": i }));

        logger.log(event).await?;
    }

    // Flush to ensure all events are written
    logger.flush().await?;

    println!("✓ Logged 3 events to file: {:?}", log_path);

    // Read and display the log file
    let content = tokio::fs::read_to_string(&log_path).await?;
    println!("  Log file contents:");
    for (i, line) in content.lines().enumerate() {
        println!("    Event {}: {}", i + 1, &line[..80.min(line.len())]);
    }

    Ok(())
}

/// Example 3: Composite writer logging to multiple destinations
async fn example_composite_logging() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::TempDir::new()?;
    let log_path = temp_dir.path().join("composite_audit.log");

    let composite = CompositeAuditWriter::new()
        .add_writer(Box::new(TracingAuditWriter::new()))
        .add_writer(Box::new(FileAuditWriter::new(log_path.clone())));

    let logger = AuditLogger::new(Box::new(composite));

    let event = AuditEvent::new(
        AuditEventType::DataModification,
        AuditActor::System,
        AuditResource::Model { id: Uuid::new_v4() },
        AuditAction::Create,
        AuditOutcome::Success,
    )
    .with_details(json!({
        "model_name": "gpt-4",
        "provider": "openai"
    }));

    logger.log(event).await?;
    logger.flush().await?;

    println!("✓ Logged to both tracing and file");
    println!("  File location: {:?}", log_path);

    Ok(())
}

/// Example 4: Logging various types of events
async fn example_event_types() -> Result<(), Box<dyn std::error::Error>> {
    let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

    let user_actor = AuditActor::User {
        id: UserId::new(),
        email: "demo@example.com".to_string(),
    };
    let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

    // Authentication event
    logger.log_auth_success(&user_actor, ip).await?;
    println!("✓ Logged authentication event");

    // Authorization event (denied)
    let auth_event = AuditEvent::new(
        AuditEventType::Authorization,
        user_actor.clone(),
        AuditResource::Experiment {
            id: Uuid::new_v4(),
        },
        AuditAction::Delete,
        AuditOutcome::Denied {
            reason: "Insufficient permissions".to_string(),
        },
    );
    logger.log(auth_event).await?;
    println!("✓ Logged authorization denial");

    // Data access event
    logger
        .log_access(
            &user_actor,
            &AuditResource::Dataset {
                id: Uuid::new_v4(),
            },
            AuditAction::Read,
            AuditOutcome::Success,
        )
        .await?;
    println!("✓ Logged data access event");

    // Data modification event
    logger
        .log_modification(
            &user_actor,
            &AuditResource::PromptTemplate {
                id: Uuid::new_v4(),
            },
            AuditAction::Update,
            Some(json!({ "template": "old template" })),
            Some(json!({ "template": "new template" })),
        )
        .await?;
    println!("✓ Logged data modification event");

    // Configuration change
    let config_event = AuditEvent::new(
        AuditEventType::Configuration,
        AuditActor::System,
        AuditResource::System,
        AuditAction::ConfigChange,
        AuditOutcome::Success,
    )
    .with_details(json!({
        "setting": "max_upload_size",
        "old_value": "10MB",
        "new_value": "50MB"
    }));
    logger.log(config_event).await?;
    println!("✓ Logged configuration change");

    // Security event
    let security_event = AuditEvent::new(
        AuditEventType::Security,
        user_actor.clone(),
        AuditResource::User {
            id: Uuid::new_v4(),
        },
        AuditAction::PasswordChange,
        AuditOutcome::Success,
    )
    .with_ip(ip);
    logger.log(security_event).await?;
    println!("✓ Logged security event");

    Ok(())
}

/// Example 5: Logging with detailed metadata
async fn example_detailed_logging() -> Result<(), Box<dyn std::error::Error>> {
    let logger = AuditLogger::new(Box::new(TracingAuditWriter::new()));

    let api_key_actor = AuditActor::ApiKey {
        id: Uuid::new_v4(),
        name: "production-api-key".to_string(),
    };

    let experiment_id = Uuid::new_v4();

    // Create a comprehensive audit event
    let event = AuditEvent::new(
        AuditEventType::DataModification,
        api_key_actor,
        AuditResource::Run {
            id: Uuid::new_v4(),
            experiment_id,
        },
        AuditAction::Create,
        AuditOutcome::Success,
    )
    .with_ip(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 42)))
    .with_user_agent("llm-research-client/1.0".to_string())
    .with_request_id(Uuid::new_v4().to_string())
    .with_duration(1250) // 1.25 seconds
    .with_details(json!({
        "run_config": {
            "model": "gpt-4-turbo",
            "temperature": 0.7,
            "max_tokens": 2000
        },
        "dataset_size": 1000,
        "estimated_cost": 15.50
    }));

    logger.log(event).await?;
    println!("✓ Logged comprehensive event with full metadata");

    // Log an anonymous access attempt
    let anon_event = AuditEvent::new(
        AuditEventType::Authorization,
        AuditActor::Anonymous {
            ip: IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10)),
        },
        AuditResource::Evaluation {
            id: Uuid::new_v4(),
        },
        AuditAction::Read,
        AuditOutcome::Denied {
            reason: "Authentication required".to_string(),
        },
    )
    .with_user_agent("curl/7.68.0".to_string());

    logger.log(anon_event).await?;
    println!("✓ Logged anonymous access attempt");

    Ok(())
}
