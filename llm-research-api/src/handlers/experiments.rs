use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use llm_research_core::domain::{
    experiment::Experiment,
    ids::{ExperimentId, UserId},
};
use uuid::Uuid;
use validator::Validate;

use crate::{dto::*, error::{ApiError, ApiResult}, AppState};

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateExperimentRequest>,
) -> ApiResult<(StatusCode, Json<ExperimentResponse>)> {
    payload.validate()?;

    let mut experiment = Experiment::new(
        payload.name,
        payload.description,
        payload.hypothesis,
        UserId::from(payload.owner_id),
        payload.config,
    );

    // Add collaborators and tags if provided
    if let Some(collaborators) = payload.collaborators {
        experiment = experiment.with_collaborators(
            collaborators.into_iter().map(UserId::from).collect()
        );
    }

    if let Some(tags) = payload.tags {
        experiment = experiment.with_tags(tags);
    }

    // TODO: Save to database using state.db_pool
    let _ = state;

    let response = ExperimentResponse::from(experiment);

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<Json<PaginatedResponse<ExperimentResponse>>> {
    pagination.validate()?;

    let limit = pagination.limit.unwrap_or(20);
    let _cursor = pagination.cursor;
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
) -> ApiResult<Json<ExperimentResponse>> {
    let _ = (state, id);

    // TODO: Fetch from database
    // let experiment_id = ExperimentId::from(id);
    Err(ApiError::NotFound("Experiment not found".to_string()))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateExperimentRequest>,
) -> ApiResult<Json<ExperimentResponse>> {
    payload.validate()?;

    let _ = (state, id);

    // TODO: Update in database
    Err(ApiError::NotFound("Experiment not found".to_string()))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let _ = (state, id);

    // TODO: Delete from database
    Ok(StatusCode::NO_CONTENT)
}

pub async fn start(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ExperimentResponse>> {
    let _ = (state, id);

    // TODO: Fetch experiment, update status to Running, save to database
    Err(ApiError::NotFound("Experiment not found".to_string()))
}

pub async fn create_run(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Json(payload): Json<CreateRunRequest>,
) -> ApiResult<(StatusCode, Json<RunResponse>)> {
    payload.validate()?;

    let _ = (state, experiment_id);

    // TODO: Create run for experiment
    let response = RunResponse {
        id: Uuid::new_v4(),
        experiment_id,
        status: "running".to_string(),
        config: payload.config_overrides.unwrap_or(serde_json::json!({})),
        started_at: Utc::now(),
        completed_at: None,
        error: None,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_runs(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<Json<PaginatedResponse<RunResponse>>> {
    pagination.validate()?;

    let _ = (state, experiment_id);

    // TODO: Fetch runs from database

    Ok(Json(PaginatedResponse {
        data: vec![],
        next_cursor: None,
        has_more: false,
        total: Some(0),
    }))
}

pub async fn complete_run(
    State(state): State<AppState>,
    Path((experiment_id, run_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<RunResponse>> {
    let _ = (state, experiment_id, run_id);

    // TODO: Update run status to Completed
    Err(ApiError::NotFound("Run not found".to_string()))
}

pub async fn fail_run(
    State(state): State<AppState>,
    Path((experiment_id, run_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<FailRunRequest>,
) -> ApiResult<Json<RunResponse>> {
    payload.validate()?;

    let _ = (state, experiment_id, run_id);

    // TODO: Update run status to Failed with error message
    Err(ApiError::NotFound("Run not found".to_string()))
}
