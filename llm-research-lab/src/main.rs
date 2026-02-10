//! LLM Research Lab - Unified Cloud Run Service
//!
//! # Constitution Compliance
//!
//! This service adheres to PROMPT 2 (RUNTIME & INFRASTRUCTURE IMPLEMENTATION):
//!
//! - **Stateless Runtime**: No local state, all persistence via ruvector-service
//! - **No Direct SQL Access**: Database access ONLY through ruvector-service
//! - **Edge Function Compatible**: Handlers are deterministic and stateless
//! - **Telemetry Emission**: LLM-Observatory compatible telemetry
//!
//! # Agentics Execution System
//!
//! This service is instrumented as a Foundational Execution Unit within the
//! Agentics execution system. Every agent endpoint:
//!
//! - Requires an `execution_context` with `execution_id` and `parent_span_id`
//! - Rejects execution if `parent_span_id` is missing
//! - Emits a repo-level span with nested agent-level spans
//! - Attaches artifacts to agent spans (never directly to Core)
//! - Returns an `ExecutionResult` envelope with the full span hierarchy
//!
//! # Service Topology
//!
//! Single unified service exposing:
//! - `/health` - Health check endpoint
//! - `/api/v1/agents/hypothesis` - Hypothesis evaluation agent
//! - `/api/v1/agents/metric` - Experimental metric computation agent
//! - `/api/v1/*` - Standard API routes

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Phase 7 Layer 2: Import RuVector client for startup validation
use llm_research_agents::{RuVectorClient, RuVectorPersistence};

// Agentics Execution System imports
use llm_research_agents::{
    ExecutionResult, ExecutionSpan,
    validate_execution_context,
};

mod config;

// Agent imports
use llm_research_agents::handlers::{
    HypothesisHandler, HypothesisEvaluateRequest, HypothesisEvaluateResponse,
    MetricHandler, MetricComputeRequest, MetricComputeResponse,
};

/// Application state for Cloud Run deployment.
///
/// CONSTITUTION COMPLIANCE: This state contains NO database pools.
/// All persistence is handled via ruvector-service through the handlers.
#[derive(Clone)]
pub struct CloudRunState {
    /// Hypothesis agent handler
    hypothesis_handler: Arc<HypothesisHandler>,
    /// Metric agent handler
    metric_handler: Arc<MetricHandler>,
    /// Service configuration
    config: Arc<config::Config>,
}

impl CloudRunState {
    /// Create new state from environment.
    ///
    /// No database connections are created here - handlers use ruvector-service.
    pub fn new(config: config::Config) -> Result<Self> {
        let hypothesis_handler = Arc::new(
            HypothesisHandler::new()
                .map_err(|e| anyhow::anyhow!("Failed to create hypothesis handler: {}", e))?
        );
        let metric_handler = Arc::new(MetricHandler::new());

        Ok(Self {
            hypothesis_handler,
            metric_handler,
            config: Arc::new(config),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    "llm_research_lab=debug,llm_research_agents=debug,tower_http=debug".into()
                }),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!(
        service = "llm-research-lab",
        version = env!("CARGO_PKG_VERSION"),
        "Starting LLM Research Lab Cloud Run service"
    );

    // Load configuration from environment
    let config = config::Config::load()?;
    info!(
        port = config.port,
        platform_env = ?config.platform_env,
        ruvector_url = %config.ruvector_service_url,
        "Configuration loaded"
    );

    // =========================================================================
    // PHASE 7 LAYER 2 MANDATORY: Validate Ruvector connectivity BEFORE serving
    // Cloud Run MUST NOT accept traffic if Ruvector is unavailable.
    // =========================================================================
    info!("Validating Ruvector connectivity...");

    let ruvector_client = RuVectorClient::from_env()
        .map_err(|e| {
            error!(
                error = %e,
                phase = "phase7",
                layer = "layer2",
                "Failed to create Ruvector client. ABORTING STARTUP."
            );
            anyhow::anyhow!("Failed to create Ruvector client: {}. ABORTING STARTUP.", e)
        })?;

    let healthy = ruvector_client.health_check().await
        .map_err(|e| {
            error!(
                error = %e,
                ruvector_url = %config.ruvector_service_url,
                phase = "phase7",
                layer = "layer2",
                "Ruvector health check failed. ABORTING STARTUP."
            );
            anyhow::anyhow!("Ruvector health check failed: {}. ABORTING STARTUP.", e)
        })?;

    if !healthy {
        error!(
            ruvector_url = %config.ruvector_service_url,
            phase = "phase7",
            layer = "layer2",
            "Ruvector service is not healthy. Cloud Run must refuse to serve traffic. ABORTING."
        );
        return Err(anyhow::anyhow!(
            "Ruvector service is not healthy. Cloud Run must refuse to serve traffic. ABORTING."
        ));
    }

    info!(
        ruvector_url = %config.ruvector_service_url,
        phase = "phase7",
        layer = "layer2",
        "Ruvector connectivity validated successfully"
    );

    // Emit Phase 7 startup log
    let agent_name = std::env::var("AGENT_NAME").unwrap_or_else(|_| "llm-research-lab".to_string());
    let agent_version = env!("CARGO_PKG_VERSION");

    info!(
        agent_name = %agent_name,
        agent_version = %agent_version,
        phase = "phase7",
        layer = "layer2",
        ruvector = true,
        "agent_started"
    );

    // Create stateless application state
    let state = CloudRunState::new(config.clone())?;
    info!("Application state initialized (stateless, no DB connections)");

    // Build router with agent endpoints
    let app = Router::new()
        // Health endpoints
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))

        // Agent endpoints (per service topology)
        .route("/api/v1/agents/hypothesis", post(hypothesis_evaluate))
        .route("/api/v1/agents/hypothesis", get(hypothesis_info))
        .route("/api/v1/agents/metric", post(metric_compute))
        .route("/api/v1/agents/metric", get(metric_info))
        .route("/api/v1/agents", get(list_agents))

        // Middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    // Phase 7 Layer 2: Structured startup completion log
    info!(
        target: "startup",
        service = "llm-research-lab",
        agent_name = %agent_name,
        agent_version = %agent_version,
        phase = "phase7",
        layer = "layer2",
        ruvector = true,
        port = config.port,
        address = %addr,
        "Service startup complete - ready to serve traffic"
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// =============================================================================
// Health Endpoints
// =============================================================================

/// Liveness probe - always returns OK if the process is running.
async fn health_check() -> &'static str {
    "OK"
}

/// Readiness probe - checks if ruvector-service is reachable.
///
/// PHASE 7 LAYER 2: This endpoint now performs an ACTUAL health check
/// against ruvector-service. Cloud Run will not route traffic to this
/// instance if Ruvector is unhealthy or unreachable.
async fn readiness_check(State(_state): State<CloudRunState>) -> (StatusCode, &'static str) {
    // Phase 7 Layer 2: Create a client and check Ruvector health
    match RuVectorClient::from_env() {
        Ok(client) => {
            match client.health_check().await {
                Ok(true) => {
                    (StatusCode::OK, "READY")
                }
                Ok(false) => {
                    warn!(
                        phase = "phase7",
                        layer = "layer2",
                        "Readiness check: Ruvector unhealthy"
                    );
                    (StatusCode::SERVICE_UNAVAILABLE, "RUVECTOR_UNHEALTHY")
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        phase = "phase7",
                        layer = "layer2",
                        "Readiness check: Ruvector unreachable"
                    );
                    (StatusCode::SERVICE_UNAVAILABLE, "RUVECTOR_UNREACHABLE")
                }
            }
        }
        Err(e) => {
            error!(
                error = %e,
                phase = "phase7",
                layer = "layer2",
                "Readiness check: Configuration error"
            );
            (StatusCode::SERVICE_UNAVAILABLE, "CONFIG_ERROR")
        }
    }
}

// =============================================================================
// Agent Endpoints (Agentics Execution System)
//
// Every agent endpoint:
// 1. Validates execution_context (rejects if parent_span_id missing)
// 2. Creates a repo-level execution span
// 3. Delegates to the handler (which creates agent-level span)
// 4. Nests the agent span under the repo span
// 5. Validates the agent-span invariant
// 6. Returns ExecutionResult envelope with full span hierarchy
//
// This repo MUST NEVER return a flat response without spans.
// This repo MUST NEVER return success if no agent spans were emitted.
// =============================================================================

/// POST /api/v1/agents/hypothesis - Evaluate a hypothesis.
///
/// Requires `execution_context` with valid `parent_span_id` from Core.
/// Returns `ExecutionResult<HypothesisEvaluateResponse>` with repo and agent spans.
async fn hypothesis_evaluate(
    State(state): State<CloudRunState>,
    Json(request): Json<HypothesisEvaluateRequest>,
) -> (StatusCode, Json<ExecutionResult<HypothesisEvaluateResponse>>) {
    // ENFORCEMENT: Validate execution context - reject if missing
    let exec_ctx = match validate_execution_context(&request.execution_context) {
        Ok(ctx) => ctx.clone(),
        Err(e) => {
            error!(
                error = %e,
                "Execution REJECTED: missing or invalid execution context"
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(ExecutionResult::rejected(e.to_string())),
            );
        }
    };

    // Create repo-level execution span (parent = Core span)
    let mut repo_span = ExecutionSpan::new_repo(exec_ctx.parent_span_id);

    // Execute handler (creates agent-level span internally)
    let (response, agent_span) = state
        .hypothesis_handler
        .handle(request, repo_span.span_id)
        .await;

    // Nest agent span under repo span
    repo_span.add_child(agent_span);

    // Finalize repo span status based on agent outcome
    if response.success {
        repo_span.complete();
    } else {
        let reason = response
            .error
            .as_ref()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "Unknown failure".to_string());
        repo_span.fail(reason);
    }

    // ENFORCEMENT: Validate agent-span invariant
    if let Err(e) = repo_span.validate_agent_spans() {
        error!(error = %e, "Execution INVALID: agent-span invariant violated");
        repo_span.fail(e.to_string());
    }

    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    (
        status,
        Json(ExecutionResult {
            execution_id: exec_ctx.execution_id,
            repo_span,
            result: Some(response),
        }),
    )
}

/// GET /api/v1/agents/hypothesis - Get hypothesis agent info.
async fn hypothesis_info() -> Json<AgentInfoResponse> {
    Json(AgentInfoResponse {
        id: "hypothesis-agent".to_string(),
        version: "1.0.0".to_string(),
        classification: "HYPOTHESIS_EVALUATION".to_string(),
        endpoint: "/api/v1/agents/hypothesis".to_string(),
        methods: vec!["POST".to_string()],
    })
}

/// POST /api/v1/agents/metric - Compute experimental metrics.
///
/// Requires `execution_context` with valid `parent_span_id` from Core.
/// Returns `ExecutionResult<MetricComputeResponse>` with repo and agent spans.
async fn metric_compute(
    State(state): State<CloudRunState>,
    Json(request): Json<MetricComputeRequest>,
) -> (StatusCode, Json<ExecutionResult<MetricComputeResponse>>) {
    // ENFORCEMENT: Validate execution context - reject if missing
    let exec_ctx = match validate_execution_context(&request.execution_context) {
        Ok(ctx) => ctx.clone(),
        Err(e) => {
            error!(
                error = %e,
                "Execution REJECTED: missing or invalid execution context"
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(ExecutionResult::rejected(e.to_string())),
            );
        }
    };

    // Create repo-level execution span (parent = Core span)
    let mut repo_span = ExecutionSpan::new_repo(exec_ctx.parent_span_id);

    // Execute handler (creates agent-level span internally)
    let (response, agent_span) = state
        .metric_handler
        .handle(request, repo_span.span_id)
        .await;

    // Nest agent span under repo span
    repo_span.add_child(agent_span);

    // Finalize repo span status based on agent outcome
    if response.success {
        repo_span.complete();
    } else {
        let reason = response
            .error
            .as_ref()
            .map(|e| e.clone())
            .unwrap_or_else(|| "Unknown failure".to_string());
        repo_span.fail(reason);
    }

    // ENFORCEMENT: Validate agent-span invariant
    if let Err(e) = repo_span.validate_agent_spans() {
        error!(error = %e, "Execution INVALID: agent-span invariant violated");
        repo_span.fail(e.to_string());
    }

    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    (
        status,
        Json(ExecutionResult {
            execution_id: exec_ctx.execution_id,
            repo_span,
            result: Some(response),
        }),
    )
}

/// GET /api/v1/agents/metric - Get metric agent info.
async fn metric_info() -> Json<AgentInfoResponse> {
    let handler = MetricHandler::new();
    let info = handler.agent_info();

    Json(AgentInfoResponse {
        id: info.id,
        version: info.version,
        classification: info.classification,
        endpoint: info.endpoint,
        methods: vec!["POST".to_string()],
    })
}

/// GET /api/v1/agents - List all available agents.
async fn list_agents() -> Json<AgentsListResponse> {
    Json(AgentsListResponse {
        agents: vec![
            AgentInfoResponse {
                id: "hypothesis-agent".to_string(),
                version: "1.0.0".to_string(),
                classification: "HYPOTHESIS_EVALUATION".to_string(),
                endpoint: "/api/v1/agents/hypothesis".to_string(),
                methods: vec!["POST".to_string()],
            },
            AgentInfoResponse {
                id: "experimental-metric-agent".to_string(),
                version: "1.0.0".to_string(),
                classification: "EXPERIMENTAL_METRICS".to_string(),
                endpoint: "/api/v1/agents/metric".to_string(),
                methods: vec!["POST".to_string()],
            },
        ],
        count: 2,
    })
}

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentInfoResponse {
    pub id: String,
    pub version: String,
    pub classification: String,
    pub endpoint: String,
    pub methods: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentsListResponse {
    pub agents: Vec<AgentInfoResponse>,
    pub count: usize,
}
