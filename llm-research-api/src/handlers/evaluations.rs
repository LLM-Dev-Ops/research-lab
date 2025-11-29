use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use llm_research_core::Evaluation;
use rust_decimal::Decimal;
use uuid::Uuid;
use validator::Validate;

use crate::{dto::*, error::{ApiError, ApiResult}, AppState};

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateEvaluationRequest>,
) -> ApiResult<(StatusCode, Json<EvaluationResponse>)> {
    payload.validate()?;

    let evaluation = Evaluation::new(
        payload.experiment_id,
        payload.sample_id,
        payload.input,
        payload.output,
        payload.expected_output,
        payload.latency_ms,
        payload.token_count,
        payload.cost,
        payload.metrics,
    );

    // TODO: Save to database using state.db_pool
    let _ = state;

    let response = EvaluationResponse {
        id: evaluation.id,
        experiment_id: evaluation.experiment_id,
        sample_id: evaluation.sample_id,
        input: evaluation.input,
        output: evaluation.output,
        expected_output: evaluation.expected_output,
        latency_ms: evaluation.latency_ms,
        token_count: evaluation.token_count,
        cost: evaluation.cost,
        metrics: evaluation.metrics,
        created_at: evaluation.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<Json<PaginatedResponse<EvaluationResponse>>> {
    pagination.validate()?;

    let _ = state;

    // TODO: Fetch from database with pagination

    Ok(Json(PaginatedResponse {
        data: vec![],
        next_cursor: None,
        has_more: false,
        total: Some(0),
    }))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<EvaluationResponse>> {
    let _ = (state, id);

    // TODO: Fetch from database
    Err(ApiError::NotFound("Evaluation not found".to_string()))
}

pub async fn get_metrics(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> ApiResult<Json<MetricsResponse>> {
    let _ = (state, experiment_id);

    // TODO: Aggregate metrics from database
    // Query all evaluations for this experiment and calculate aggregates

    let response = MetricsResponse {
        experiment_id,
        total_samples: 0,
        avg_latency_ms: 0.0,
        total_tokens: 0,
        total_cost: Some(Decimal::ZERO),
        accuracy: None,
        custom_metrics: serde_json::json!({}),
    };

    Ok(Json(response))
}
