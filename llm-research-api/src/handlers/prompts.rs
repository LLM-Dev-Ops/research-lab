use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use llm_research_core::PromptTemplate;
use uuid::Uuid;
use validator::Validate;

use crate::{dto::*, error::{ApiError, ApiResult}, AppState};

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreatePromptTemplateRequest>,
) -> ApiResult<(StatusCode, Json<PromptTemplateResponse>)> {
    payload.validate()?;

    let template = PromptTemplate::new(
        payload.name,
        payload.description,
        payload.template,
    );

    // TODO: Save to database using state.db_pool
    let _ = state;

    let response = PromptTemplateResponse {
        id: template.id,
        name: template.name,
        description: template.description,
        template: template.template,
        variables: template.variables,
        version: template.version,
        created_at: template.created_at,
        updated_at: template.updated_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<Json<PaginatedResponse<PromptTemplateResponse>>> {
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
) -> ApiResult<Json<PromptTemplateResponse>> {
    let _ = (state, id);

    // TODO: Fetch from database
    Err(ApiError::NotFound("Prompt template not found".to_string()))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePromptTemplateRequest>,
) -> ApiResult<Json<PromptTemplateResponse>> {
    payload.validate()?;

    let _ = (state, id);

    // TODO: Update in database
    Err(ApiError::NotFound("Prompt template not found".to_string()))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let _ = (state, id);

    // TODO: Delete from database
    Ok(StatusCode::NO_CONTENT)
}
