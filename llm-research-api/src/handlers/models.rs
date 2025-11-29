use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use llm_research_core::domain::model::{Model, ModelProvider};
use uuid::Uuid;
use validator::Validate;

use crate::{dto::*, error::{ApiError, ApiResult}, AppState};

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateModelRequest>,
) -> ApiResult<(StatusCode, Json<ModelResponse>)> {
    payload.validate()?;

    let model = Model::new(
        payload.name,
        payload.provider,
        payload.model_identifier,
        payload.version,
        payload.config,
    );

    // TODO: Save to database using state.db_pool
    let _ = state;

    let response = ModelResponse::from(model);

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<Json<PaginatedResponse<ModelResponse>>> {
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
) -> ApiResult<Json<ModelResponse>> {
    let _ = (state, id);

    // TODO: Fetch from database
    Err(ApiError::NotFound("Model not found".to_string()))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateModelRequest>,
) -> ApiResult<Json<ModelResponse>> {
    payload.validate()?;

    let _ = (state, id);

    // TODO: Update in database
    Err(ApiError::NotFound("Model not found".to_string()))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let _ = (state, id);

    // TODO: Delete from database
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_providers() -> ApiResult<Json<Vec<ProviderResponse>>> {
    let providers = vec![
        ProviderResponse {
            name: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            description: Some("OpenAI GPT models".to_string()),
            supported_models: vec![
                "gpt-4".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
        },
        ProviderResponse {
            name: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            description: Some("Anthropic Claude models".to_string()),
            supported_models: vec![
                "claude-3-opus".to_string(),
                "claude-3-sonnet".to_string(),
                "claude-3-haiku".to_string(),
            ],
        },
        ProviderResponse {
            name: "google".to_string(),
            display_name: "Google".to_string(),
            description: Some("Google Gemini models".to_string()),
            supported_models: vec![
                "gemini-pro".to_string(),
                "gemini-ultra".to_string(),
            ],
        },
        ProviderResponse {
            name: "cohere".to_string(),
            display_name: "Cohere".to_string(),
            description: Some("Cohere language models".to_string()),
            supported_models: vec![
                "command".to_string(),
                "command-light".to_string(),
            ],
        },
        ProviderResponse {
            name: "huggingface".to_string(),
            display_name: "HuggingFace".to_string(),
            description: Some("HuggingFace hosted models".to_string()),
            supported_models: vec![
                "meta-llama/Llama-2-70b".to_string(),
                "mistralai/Mistral-7B".to_string(),
            ],
        },
        ProviderResponse {
            name: "azure".to_string(),
            display_name: "Azure OpenAI".to_string(),
            description: Some("Azure OpenAI Service".to_string()),
            supported_models: vec![
                "gpt-4".to_string(),
                "gpt-35-turbo".to_string(),
            ],
        },
        ProviderResponse {
            name: "aws".to_string(),
            display_name: "AWS Bedrock".to_string(),
            description: Some("AWS Bedrock models".to_string()),
            supported_models: vec![
                "anthropic.claude-v2".to_string(),
                "amazon.titan-text".to_string(),
            ],
        },
        ProviderResponse {
            name: "local".to_string(),
            display_name: "Local".to_string(),
            description: Some("Locally hosted models".to_string()),
            supported_models: vec![],
        },
        ProviderResponse {
            name: "custom".to_string(),
            display_name: "Custom".to_string(),
            description: Some("Custom model provider".to_string()),
            supported_models: vec![],
        },
    ];

    Ok(Json(providers))
}
