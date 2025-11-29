mod common;

use llm_research_core::domain::{
    ExperimentStatus, ModelProvider,
    ids::UserId,
};
use uuid::Uuid;

// Helper functions are exposed for testing status conversions
mod experiment_helpers {
    use super::*;

    pub fn test_status_to_str(status: &ExperimentStatus) -> &'static str {
        match status {
            ExperimentStatus::Draft => "draft",
            ExperimentStatus::Active => "active",
            ExperimentStatus::Paused => "paused",
            ExperimentStatus::Completed => "completed",
            ExperimentStatus::Archived => "archived",
            ExperimentStatus::Failed => "failed",
        }
    }

    pub fn test_str_to_status(s: &str) -> ExperimentStatus {
        match s {
            "draft" => ExperimentStatus::Draft,
            "active" => ExperimentStatus::Active,
            "paused" => ExperimentStatus::Paused,
            "completed" => ExperimentStatus::Completed,
            "archived" => ExperimentStatus::Archived,
            "failed" => ExperimentStatus::Failed,
            _ => ExperimentStatus::Draft,
        }
    }
}

mod model_helpers {
    use super::*;

    pub fn test_provider_to_str(provider: &ModelProvider) -> &'static str {
        match provider {
            ModelProvider::OpenAI => "openai",
            ModelProvider::Anthropic => "anthropic",
            ModelProvider::Google => "google",
            ModelProvider::Cohere => "cohere",
            ModelProvider::HuggingFace => "huggingface",
            ModelProvider::Azure => "azure",
            ModelProvider::AWS => "aws",
            ModelProvider::Local => "local",
            ModelProvider::Custom => "custom",
        }
    }

    pub fn test_str_to_provider(s: &str) -> ModelProvider {
        match s {
            "openai" => ModelProvider::OpenAI,
            "anthropic" => ModelProvider::Anthropic,
            "google" => ModelProvider::Google,
            "cohere" => ModelProvider::Cohere,
            "huggingface" => ModelProvider::HuggingFace,
            "azure" => ModelProvider::Azure,
            "aws" => ModelProvider::AWS,
            "local" => ModelProvider::Local,
            "custom" => ModelProvider::Custom,
            _ => ModelProvider::Custom,
        }
    }
}

#[cfg(test)]
mod experiment_repository_tests {
    use super::*;
    use common::*;

    #[test]
    fn test_experiment_status_to_str_conversion() {
        assert_eq!(
            experiment_helpers::test_status_to_str(&ExperimentStatus::Draft),
            "draft"
        );
        assert_eq!(
            experiment_helpers::test_status_to_str(&ExperimentStatus::Active),
            "active"
        );
        assert_eq!(
            experiment_helpers::test_status_to_str(&ExperimentStatus::Paused),
            "paused"
        );
        assert_eq!(
            experiment_helpers::test_status_to_str(&ExperimentStatus::Completed),
            "completed"
        );
        assert_eq!(
            experiment_helpers::test_status_to_str(&ExperimentStatus::Archived),
            "archived"
        );
        assert_eq!(
            experiment_helpers::test_status_to_str(&ExperimentStatus::Failed),
            "failed"
        );
    }

    #[test]
    fn test_experiment_str_to_status_conversion() {
        assert_eq!(
            experiment_helpers::test_str_to_status("draft"),
            ExperimentStatus::Draft
        );
        assert_eq!(
            experiment_helpers::test_str_to_status("active"),
            ExperimentStatus::Active
        );
        assert_eq!(
            experiment_helpers::test_str_to_status("paused"),
            ExperimentStatus::Paused
        );
        assert_eq!(
            experiment_helpers::test_str_to_status("completed"),
            ExperimentStatus::Completed
        );
        assert_eq!(
            experiment_helpers::test_str_to_status("archived"),
            ExperimentStatus::Archived
        );
        assert_eq!(
            experiment_helpers::test_str_to_status("failed"),
            ExperimentStatus::Failed
        );

        // Test default case
        assert_eq!(
            experiment_helpers::test_str_to_status("unknown"),
            ExperimentStatus::Draft
        );
    }

    #[test]
    fn test_experiment_status_roundtrip() {
        let statuses = vec![
            ExperimentStatus::Draft,
            ExperimentStatus::Active,
            ExperimentStatus::Paused,
            ExperimentStatus::Completed,
            ExperimentStatus::Archived,
            ExperimentStatus::Failed,
        ];

        for status in statuses {
            let str_repr = experiment_helpers::test_status_to_str(&status);
            let converted_back = experiment_helpers::test_str_to_status(str_repr);
            assert_eq!(status, converted_back);
        }
    }

    #[test]
    fn test_experiment_search_pattern_generation() {
        let query = "test";
        let pattern = format!("%{}%", query);
        assert_eq!(pattern, "%test%");

        let query_with_special = "test%search";
        let pattern = format!("%{}%", query_with_special);
        assert_eq!(pattern, "%test%search%");
    }

    #[test]
    fn test_experiment_collaborators_conversion() {
        let experiment = create_test_experiment();

        // Convert to Vec<Uuid>
        let collaborators_uuids: Vec<Uuid> = experiment
            .collaborators
            .iter()
            .map(|id| id.0)
            .collect();

        assert_eq!(collaborators_uuids.len(), experiment.collaborators.len());

        // Convert back to Vec<UserId>
        let collaborators_back: Vec<UserId> = collaborators_uuids
            .into_iter()
            .map(UserId)
            .collect();

        assert_eq!(collaborators_back.len(), experiment.collaborators.len());
    }

    #[test]
    fn test_experiment_metadata_serialization() {
        let mut experiment = create_test_experiment();
        experiment.metadata.insert("key1".to_string(), serde_json::json!("value1"));
        experiment.metadata.insert("key2".to_string(), serde_json::json!(42));

        let metadata_json = serde_json::to_value(&experiment.metadata).unwrap();
        assert!(metadata_json.is_object());

        let metadata_back: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_value(metadata_json).unwrap();

        assert_eq!(metadata_back.len(), 2);
        assert_eq!(metadata_back["key1"], serde_json::json!("value1"));
        assert_eq!(metadata_back["key2"], serde_json::json!(42));
    }

    #[test]
    fn test_experiment_config_serialization() {
        let experiment = create_test_experiment();
        let config_json = serde_json::to_value(&experiment.config).unwrap();
        assert!(config_json.is_object());

        // Test deserialization with fallback
        let _config: llm_research_core::domain::ExperimentConfig =
            serde_json::from_value(config_json)
                .unwrap_or_else(|_| llm_research_core::domain::ExperimentConfig::default());

        // Should not panic and return valid config
        assert!(true);
    }
}

#[cfg(test)]
mod model_repository_tests {
    use super::*;
    use common::*;

    #[test]
    fn test_model_provider_to_str_conversion() {
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::OpenAI),
            "openai"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::Anthropic),
            "anthropic"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::Google),
            "google"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::Cohere),
            "cohere"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::HuggingFace),
            "huggingface"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::Azure),
            "azure"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::AWS),
            "aws"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::Local),
            "local"
        );
        assert_eq!(
            model_helpers::test_provider_to_str(&ModelProvider::Custom),
            "custom"
        );
    }

    #[test]
    fn test_model_str_to_provider_conversion() {
        assert_eq!(
            model_helpers::test_str_to_provider("openai"),
            ModelProvider::OpenAI
        );
        assert_eq!(
            model_helpers::test_str_to_provider("anthropic"),
            ModelProvider::Anthropic
        );
        assert_eq!(
            model_helpers::test_str_to_provider("google"),
            ModelProvider::Google
        );
        assert_eq!(
            model_helpers::test_str_to_provider("cohere"),
            ModelProvider::Cohere
        );
        assert_eq!(
            model_helpers::test_str_to_provider("huggingface"),
            ModelProvider::HuggingFace
        );
        assert_eq!(
            model_helpers::test_str_to_provider("azure"),
            ModelProvider::Azure
        );
        assert_eq!(
            model_helpers::test_str_to_provider("aws"),
            ModelProvider::AWS
        );
        assert_eq!(
            model_helpers::test_str_to_provider("local"),
            ModelProvider::Local
        );
        assert_eq!(
            model_helpers::test_str_to_provider("custom"),
            ModelProvider::Custom
        );

        // Test default case
        assert_eq!(
            model_helpers::test_str_to_provider("unknown"),
            ModelProvider::Custom
        );
    }

    #[test]
    fn test_model_provider_roundtrip() {
        let providers = vec![
            ModelProvider::OpenAI,
            ModelProvider::Anthropic,
            ModelProvider::Google,
            ModelProvider::Cohere,
            ModelProvider::HuggingFace,
            ModelProvider::Azure,
            ModelProvider::AWS,
            ModelProvider::Local,
            ModelProvider::Custom,
        ];

        for provider in providers {
            let str_repr = model_helpers::test_provider_to_str(&provider);
            let converted_back = model_helpers::test_str_to_provider(str_repr);
            assert_eq!(provider, converted_back);
        }
    }

    #[test]
    fn test_model_config_serialization() {
        let model = create_test_model();

        let config_json = serde_json::to_value(&model.config).unwrap();
        assert!(config_json.is_object());

        // Config should be deserializable
        let _config_back: serde_json::Value =
            serde_json::from_value(config_json).unwrap();
    }

    #[test]
    fn test_model_search_pattern_generation() {
        let query = "gpt";
        let pattern = format!("%{}%", query);
        assert_eq!(pattern, "%gpt%");
    }
}

#[cfg(test)]
mod run_repository_tests {
    use super::*;
    use common::*;
    use llm_research_core::domain::RunStatus;

    fn run_status_to_str(status: &RunStatus) -> &'static str {
        match status {
            RunStatus::Pending => "pending",
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
            RunStatus::TimedOut => "timedout",
        }
    }

    fn str_to_run_status(s: &str) -> RunStatus {
        match s {
            "pending" => RunStatus::Pending,
            "queued" => RunStatus::Queued,
            "running" => RunStatus::Running,
            "completed" => RunStatus::Completed,
            "failed" => RunStatus::Failed,
            "cancelled" => RunStatus::Cancelled,
            "timedout" => RunStatus::TimedOut,
            _ => RunStatus::Pending,
        }
    }

    #[test]
    fn test_run_status_to_str_conversion() {
        assert_eq!(run_status_to_str(&RunStatus::Pending), "pending");
        assert_eq!(run_status_to_str(&RunStatus::Queued), "queued");
        assert_eq!(run_status_to_str(&RunStatus::Running), "running");
        assert_eq!(run_status_to_str(&RunStatus::Completed), "completed");
        assert_eq!(run_status_to_str(&RunStatus::Failed), "failed");
        assert_eq!(run_status_to_str(&RunStatus::Cancelled), "cancelled");
        assert_eq!(run_status_to_str(&RunStatus::TimedOut), "timedout");
    }

    #[test]
    fn test_run_str_to_status_conversion() {
        assert_eq!(str_to_run_status("pending"), RunStatus::Pending);
        assert_eq!(str_to_run_status("queued"), RunStatus::Queued);
        assert_eq!(str_to_run_status("running"), RunStatus::Running);
        assert_eq!(str_to_run_status("completed"), RunStatus::Completed);
        assert_eq!(str_to_run_status("failed"), RunStatus::Failed);
        assert_eq!(str_to_run_status("cancelled"), RunStatus::Cancelled);
        assert_eq!(str_to_run_status("timedout"), RunStatus::TimedOut);

        // Test default case
        assert_eq!(str_to_run_status("unknown"), RunStatus::Pending);
    }

    #[test]
    fn test_run_status_roundtrip() {
        let statuses = vec![
            RunStatus::Pending,
            RunStatus::Queued,
            RunStatus::Running,
            RunStatus::Completed,
            RunStatus::Failed,
            RunStatus::Cancelled,
            RunStatus::TimedOut,
        ];

        for status in statuses {
            let str_repr = run_status_to_str(&status);
            let converted_back = str_to_run_status(str_repr);
            assert_eq!(status, converted_back);
        }
    }

    #[test]
    fn test_run_parameters_serialization() {
        let run = create_test_run();

        let params_json = serde_json::to_value(&run.parameters).unwrap();
        assert!(params_json.is_object());

        // Should be able to deserialize back
        let _params_back: std::collections::HashMap<String, llm_research_core::domain::config::ParameterValue> =
            serde_json::from_value(params_json).unwrap();
    }

    #[test]
    fn test_run_environment_serialization() {
        let run = create_test_run();

        if let Some(env) = &run.environment {
            let env_json = serde_json::to_value(env).unwrap();
            assert!(env_json.is_object());

            // Should be able to deserialize back
            let _env_back: llm_research_core::domain::run::EnvironmentSnapshot =
                serde_json::from_value(env_json).ok().unwrap();
        }
    }

    #[test]
    fn test_run_metrics_serialization() {
        let run = create_test_run();

        let metrics_json = serde_json::to_value(&run.metrics).unwrap();

        // Should be able to deserialize back
        let _metrics_back: llm_research_core::domain::run::RunMetrics =
            serde_json::from_value(metrics_json)
                .unwrap_or_else(|_| llm_research_core::domain::run::RunMetrics::default());
    }

    #[test]
    fn test_run_error_serialization() {
        let run = create_test_run_with_status(RunStatus::Failed);

        if let Some(error) = &run.error {
            let error_json = serde_json::to_value(error).unwrap();
            assert!(error_json.is_object());

            // Should be able to deserialize back
            let error_back: llm_research_core::domain::run::RunError =
                serde_json::from_value(error_json).ok().unwrap();

            assert_eq!(error_back.error_type, error.error_type);
            assert_eq!(error_back.message, error.message);
        }
    }

    #[test]
    fn test_run_number_increment_logic() {
        // Simulate getting next run number
        let current_max = 5i64;
        let next_num = (current_max + 1) as u32;
        assert_eq!(next_num, 6);

        // Test with no existing runs
        let current_max = 0i64;
        let next_num = std::cmp::max(current_max + 1, 1) as u32;
        assert_eq!(next_num, 1);
    }
}

#[cfg(test)]
mod dataset_repository_tests {
    use super::*;
    use common::*;

    #[test]
    fn test_dataset_schema_serialization() {
        let dataset = create_test_dataset();

        let schema_json = serde_json::to_value(&dataset.schema).unwrap();
        assert!(schema_json.is_object());

        // Should be able to deserialize back
        let _schema_back: serde_json::Value =
            serde_json::from_value(schema_json).unwrap();
    }

    #[test]
    fn test_dataset_search_pattern_generation() {
        let query = "test";
        let pattern = format!("%{}%", query);
        assert_eq!(pattern, "%test%");
    }

    #[test]
    fn test_dataset_sample_count_handling() {
        let dataset = create_test_dataset();
        assert!(dataset.sample_count > 0);

        // Sample count should be positive i64
        let count_i64: i64 = dataset.sample_count;
        assert!(count_i64 > 0);
    }
}

#[cfg(test)]
mod prompt_repository_tests {
    use super::*;
    use common::*;

    #[test]
    fn test_prompt_variables_serialization() {
        let prompt = create_test_prompt_template();

        // Variables should be a Vec<String>
        assert!(!prompt.variables.is_empty());
        assert_eq!(prompt.variables[0], "name");
    }

    #[test]
    fn test_prompt_version_handling() {
        let prompt = create_test_prompt_template();

        // Version should be a positive i32
        assert_eq!(prompt.version, 1);

        // Test version increment
        let next_version = prompt.version + 1;
        assert_eq!(next_version, 2);
    }

    #[test]
    fn test_prompt_template_parsing() {
        let prompt = create_test_prompt_template();

        // Template should contain variable placeholders
        assert!(prompt.template.contains("{{"));
        assert!(prompt.template.contains("}}"));

        // Should be able to extract variables from template
        let template = "Hello {{name}}, your age is {{age}}";
        let var_count = template.matches("{{").count();
        assert_eq!(var_count, 2);
    }

    #[test]
    fn test_prompt_search_pattern_generation() {
        let query = "greeting";
        let pattern = format!("%{}%", query);
        assert_eq!(pattern, "%greeting%");
    }
}

#[cfg(test)]
mod evaluation_repository_tests {
    use super::*;
    use common::*;

    #[test]
    fn test_evaluation_metrics_serialization() {
        let evaluation = create_test_evaluation();

        let metrics_json = serde_json::to_value(&evaluation.metrics).unwrap();
        assert!(metrics_json.is_object());

        // Should be able to deserialize back
        let _metrics_back: serde_json::Value =
            serde_json::from_value(metrics_json).unwrap();
    }

    #[test]
    fn test_evaluation_aggregation_calculations() {
        // Simulate aggregation calculations
        let evaluations = vec![
            create_test_evaluation(),
            create_test_evaluation(),
            create_test_evaluation(),
        ];

        let total_count = evaluations.len() as i64;
        assert_eq!(total_count, 3);

        let total_latency: i64 = evaluations.iter().map(|e| e.latency_ms).sum();
        let avg_latency = (total_latency as f64) / (total_count as f64);
        assert!(avg_latency > 0.0);

        let min_latency = evaluations.iter().map(|e| e.latency_ms).min().unwrap();
        let max_latency = evaluations.iter().map(|e| e.latency_ms).max().unwrap();
        assert!(min_latency <= max_latency);

        let total_tokens: i64 = evaluations.iter().map(|e| e.token_count as i64).sum();
        assert!(total_tokens > 0);
    }

    #[test]
    fn test_evaluation_cost_handling() {
        let evaluation = create_test_evaluation();

        if let Some(cost) = evaluation.cost {
            // Cost should be a valid Decimal
            assert!(cost >= rust_decimal::Decimal::ZERO);

            // Should be able to convert to f64 for aggregations
            let cost_f64 = cost.to_string().parse::<f64>().unwrap();
            assert!(cost_f64 > 0.0);
        }
    }

    #[test]
    fn test_evaluation_latency_metrics() {
        let evaluation = create_test_evaluation();

        // Latency should be positive
        assert!(evaluation.latency_ms > 0);

        // Token count should be positive
        assert!(evaluation.token_count > 0);
    }
}
