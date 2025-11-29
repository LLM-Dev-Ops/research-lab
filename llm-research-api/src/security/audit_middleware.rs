//! Axum middleware for automatic audit logging of HTTP requests

use super::audit::{
    AuditAction, AuditActor, AuditEvent, AuditEventType, AuditLogger, AuditOutcome,
    AuditResource, AuditResult,
};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::IpAddr;
use std::time::Instant;
use uuid::Uuid;

/// Middleware for auditing HTTP requests
pub async fn audit_middleware(
    State(logger): State<AuditLogger>,
    request: Request,
    next: Next,
) -> Result<Response, AuditMiddlewareError> {
    let start = Instant::now();

    // Extract request metadata
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = Uuid::new_v4().to_string();

    // Extract IP address from headers or connection info
    let ip_address = extract_ip_address(&request);

    // Extract user agent
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Determine actor from request (this would typically be extracted from JWT or API key)
    // For this example, we'll use System as a placeholder
    let actor = AuditActor::System;

    // Determine resource and action from the request path and method
    let (resource, action) = extract_resource_and_action(&method, uri.path());

    // Process the request
    let response = next.run(request).await;

    // Determine outcome from response status
    let outcome = match response.status() {
        status if status.is_success() => AuditOutcome::Success,
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => AuditOutcome::Denied {
            reason: "Access denied".to_string(),
        },
        status => AuditOutcome::Failure {
            reason: format!("HTTP {}", status.as_u16()),
        },
    };

    // Calculate duration
    let duration_ms = start.elapsed().as_millis() as u64;

    // Create and log audit event
    let mut event = AuditEvent::new(
        AuditEventType::DataAccess,
        actor,
        resource,
        action,
        outcome,
    )
    .with_request_id(request_id)
    .with_duration(duration_ms);

    if let Some(ip) = ip_address {
        event = event.with_ip(ip);
    }

    if let Some(ua) = user_agent {
        event = event.with_user_agent(ua);
    }

    // Log the event (don't fail the request if logging fails)
    if let Err(e) = logger.log(event).await {
        tracing::error!("Failed to log audit event: {}", e);
    }

    Ok(response)
}

/// Extract IP address from request
fn extract_ip_address(request: &Request) -> Option<IpAddr> {
    // Try X-Forwarded-For header first
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return Some(ip);
            }
        }
    }

    // Fall back to connection info (would need to be passed through extensions)
    None
}

/// Extract resource and action from HTTP method and path
fn extract_resource_and_action(method: &Method, path: &str) -> (AuditResource, AuditAction) {
    let action = match method {
        &Method::GET => AuditAction::Read,
        &Method::POST => AuditAction::Create,
        &Method::PUT | &Method::PATCH => AuditAction::Update,
        &Method::DELETE => AuditAction::Delete,
        _ => AuditAction::Read,
    };

    // Parse path to determine resource
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

    let resource = match parts.first() {
        Some(&"experiments") => {
            if let Some(id_str) = parts.get(1) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    AuditResource::Experiment { id }
                } else {
                    AuditResource::System
                }
            } else {
                AuditResource::System
            }
        }
        Some(&"models") => {
            if let Some(id_str) = parts.get(1) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    AuditResource::Model { id }
                } else {
                    AuditResource::System
                }
            } else {
                AuditResource::System
            }
        }
        Some(&"datasets") => {
            if let Some(id_str) = parts.get(1) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    AuditResource::Dataset { id }
                } else {
                    AuditResource::System
                }
            } else {
                AuditResource::System
            }
        }
        Some(&"prompts") => {
            if let Some(id_str) = parts.get(1) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    AuditResource::PromptTemplate { id }
                } else {
                    AuditResource::System
                }
            } else {
                AuditResource::System
            }
        }
        Some(&"evaluations") => {
            if let Some(id_str) = parts.get(1) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    AuditResource::Evaluation { id }
                } else {
                    AuditResource::System
                }
            } else {
                AuditResource::System
            }
        }
        Some(&"users") => {
            if let Some(id_str) = parts.get(1) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    AuditResource::User { id }
                } else {
                    AuditResource::System
                }
            } else {
                AuditResource::System
            }
        }
        _ => AuditResource::System,
    };

    (resource, action)
}

/// Error type for audit middleware
#[derive(Debug, thiserror::Error)]
pub enum AuditMiddlewareError {
    #[error("Audit error: {0}")]
    Audit(#[from] super::audit::AuditError),
}

impl IntoResponse for AuditMiddlewareError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resource_and_action() {
        let (resource, action) = extract_resource_and_action(&Method::GET, "/experiments");
        assert!(matches!(resource, AuditResource::System));
        assert_eq!(action, AuditAction::Read);

        let id = Uuid::new_v4();
        let (resource, action) = extract_resource_and_action(&Method::GET, &format!("/experiments/{}", id));
        match resource {
            AuditResource::Experiment { id: res_id } => assert_eq!(res_id, id),
            _ => panic!("Expected Experiment resource"),
        }
        assert_eq!(action, AuditAction::Read);

        let (_, action) = extract_resource_and_action(&Method::POST, "/experiments");
        assert_eq!(action, AuditAction::Create);

        let (_, action) = extract_resource_and_action(&Method::PUT, "/experiments/123");
        assert_eq!(action, AuditAction::Update);

        let (_, action) = extract_resource_and_action(&Method::DELETE, "/experiments/123");
        assert_eq!(action, AuditAction::Delete);
    }
}
