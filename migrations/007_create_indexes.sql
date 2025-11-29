-- Additional performance indexes and optimizations

-- Create evaluations table for storing evaluation results
CREATE TABLE IF NOT EXISTS evaluations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    experiment_run_id UUID NOT NULL REFERENCES experiment_runs(id) ON DELETE CASCADE,
    sample_id UUID NOT NULL,
    input TEXT NOT NULL,
    output TEXT NOT NULL,
    expected_output TEXT,
    latency_ms BIGINT NOT NULL,
    token_count INTEGER NOT NULL DEFAULT 0,
    cost DECIMAL(12, 6),
    metrics JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT evaluations_latency_nonnegative CHECK (latency_ms >= 0),
    CONSTRAINT evaluations_token_count_nonnegative CHECK (token_count >= 0)
);

-- Create indexes for evaluations
CREATE INDEX idx_evaluations_run_id ON evaluations(experiment_run_id);
CREATE INDEX idx_evaluations_sample_id ON evaluations(sample_id);
CREATE INDEX idx_evaluations_created_at ON evaluations(created_at DESC);
CREATE INDEX idx_evaluations_latency ON evaluations(latency_ms);
CREATE INDEX idx_evaluations_token_count ON evaluations(token_count);
CREATE INDEX idx_evaluations_metrics ON evaluations USING GIN(metrics);

-- Composite indexes for common query patterns
CREATE INDEX idx_experiments_status_created ON experiments(status, created_at DESC);
CREATE INDEX idx_experiments_owner_status ON experiments(owner_id, status);
CREATE INDEX idx_experiment_runs_exp_status ON experiment_runs(experiment_id, status);
CREATE INDEX idx_metric_values_run_metric_recorded ON metric_values(experiment_run_id, metric_definition_id, recorded_at DESC);

-- Foreign key indexes for experiments (forward references)
CREATE INDEX idx_experiments_model_fk ON experiments(model_id);
CREATE INDEX idx_experiments_dataset_fk ON experiments(dataset_id);
CREATE INDEX idx_experiments_prompt_fk ON experiments(prompt_template_id);

-- Create materialized view for experiment statistics
CREATE MATERIALIZED VIEW IF NOT EXISTS experiment_statistics AS
SELECT
    e.id as experiment_id,
    e.name as experiment_name,
    e.status,
    COUNT(DISTINCT er.id) as total_runs,
    COUNT(DISTINCT er.id) FILTER (WHERE er.status = 'completed') as completed_runs,
    COUNT(DISTINCT er.id) FILTER (WHERE er.status = 'failed') as failed_runs,
    SUM(er.total_samples) as total_samples_processed,
    SUM(er.total_tokens) as total_tokens_used,
    SUM(er.total_cost) as total_cost,
    AVG(er.duration_ms) as avg_run_duration_ms,
    MAX(er.updated_at) as last_run_at
FROM experiments e
LEFT JOIN experiment_runs er ON e.id = er.experiment_id
GROUP BY e.id, e.name, e.status;

-- Create index on materialized view
CREATE INDEX idx_experiment_statistics_experiment_id ON experiment_statistics(experiment_id);
CREATE INDEX idx_experiment_statistics_status ON experiment_statistics(status);

-- Create function to refresh experiment statistics
CREATE OR REPLACE FUNCTION refresh_experiment_statistics()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY experiment_statistics;
END;
$$ LANGUAGE plpgsql;

-- Add foreign key constraints to experiments (deferred to this migration for proper table ordering)
ALTER TABLE experiments
    ADD CONSTRAINT fk_experiments_model
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE RESTRICT;

ALTER TABLE experiments
    ADD CONSTRAINT fk_experiments_dataset
    FOREIGN KEY (dataset_id) REFERENCES datasets(id) ON DELETE RESTRICT;

ALTER TABLE experiments
    ADD CONSTRAINT fk_experiments_prompt_template
    FOREIGN KEY (prompt_template_id) REFERENCES prompt_templates(id) ON DELETE RESTRICT;

-- Create partial indexes for active records
CREATE INDEX idx_active_experiments ON experiments(created_at DESC)
    WHERE status IN ('draft', 'running');

CREATE INDEX idx_active_runs ON experiment_runs(created_at DESC)
    WHERE status IN ('pending', 'running');

-- Create index for full-text search on experiment names and descriptions
CREATE INDEX idx_experiments_search ON experiments
    USING GIN(to_tsvector('english', name || ' ' || COALESCE(description, '')));

-- Add comments for documentation
COMMENT ON TABLE experiments IS 'Stores experiment configurations and metadata';
COMMENT ON TABLE experiment_runs IS 'Stores individual experiment run executions';
COMMENT ON TABLE datasets IS 'Stores dataset metadata and references';
COMMENT ON TABLE models IS 'Stores LLM model configurations';
COMMENT ON TABLE prompt_templates IS 'Stores prompt templates with version control';
COMMENT ON TABLE artifacts IS 'Stores experiment artifacts and outputs';
COMMENT ON TABLE metric_definitions IS 'Stores metric type definitions';
COMMENT ON TABLE metric_values IS 'Stores actual metric measurements';
COMMENT ON TABLE evaluations IS 'Stores evaluation results for experiment runs';
