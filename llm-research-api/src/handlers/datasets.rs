use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use llm_research_core::Dataset;
use uuid::Uuid;
use validator::Validate;

use crate::{dto::*, error::{ApiError, ApiResult}, AppState};

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateDatasetRequest>,
) -> ApiResult<(StatusCode, Json<DatasetResponse>)> {
    payload.validate()?;

    let dataset = Dataset::new(
        payload.name,
        payload.description,
        payload.s3_path,
        0, // sample_count will be updated after upload
        payload.schema,
    );

    // TODO: Save to database using state.db_pool
    let _ = state;

    let response = DatasetResponse {
        id: dataset.id,
        name: dataset.name,
        description: dataset.description,
        s3_path: dataset.s3_path,
        sample_count: dataset.sample_count,
        schema: dataset.schema,
        created_at: dataset.created_at,
        updated_at: dataset.updated_at,
        version: Some(1),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<Json<PaginatedResponse<DatasetResponse>>> {
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
) -> ApiResult<Json<DatasetResponse>> {
    let _ = (state, id);

    // TODO: Fetch from database
    Err(ApiError::NotFound("Dataset not found".to_string()))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDatasetRequest>,
) -> ApiResult<Json<DatasetResponse>> {
    payload.validate()?;

    let _ = (state, id);

    // TODO: Update in database
    Err(ApiError::NotFound("Dataset not found".to_string()))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let _ = (state, id);

    // TODO: Delete from database and S3
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_version(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(payload): Json<CreateDatasetVersionRequest>,
) -> ApiResult<(StatusCode, Json<DatasetVersionResponse>)> {
    payload.validate()?;

    let _ = (state, dataset_id);

    // TODO: Create new version in database
    let response = DatasetVersionResponse {
        version: 2,
        dataset_id,
        s3_path: payload.s3_path,
        sample_count: 0,
        created_at: Utc::now(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_versions(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<Json<PaginatedResponse<DatasetVersionResponse>>> {
    pagination.validate()?;

    let _ = (state, dataset_id);

    // TODO: Fetch versions from database

    Ok(Json(PaginatedResponse {
        data: vec![],
        next_cursor: None,
        has_more: false,
        total: Some(0),
    }))
}

pub async fn upload(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> ApiResult<Json<UploadUrlResponse>> {
    let _ = (state, dataset_id);

    // TODO: Generate presigned S3 upload URL
    let s3_path = format!("datasets/{}/data.jsonl", dataset_id);
    let upload_url = format!("https://s3.amazonaws.com/presigned-upload-url");

    Ok(Json(UploadUrlResponse {
        upload_url,
        s3_path,
    }))
}

pub async fn download(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> ApiResult<Json<DownloadUrlResponse>> {
    let _ = (state, dataset_id);

    // TODO: Generate presigned S3 download URL
    let download_url = format!("https://s3.amazonaws.com/presigned-download-url");

    Ok(Json(DownloadUrlResponse {
        download_url,
    }))
}
