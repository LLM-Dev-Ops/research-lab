-- Create metric type enum
CREATE TYPE metric_type AS ENUM ('counter', 'gauge', 'histogram', 'summary');

-- Create metric_definitions table
CREATE TABLE IF NOT EXISTS metric_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    metric_type metric_type NOT NULL,
    unit VARCHAR(50),
    aggregation_method VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT metric_definitions_name_length CHECK (char_length(name) > 0 AND char_length(name) <= 255)
);

-- Create metric_values table
CREATE TABLE IF NOT EXISTS metric_values (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_definition_id UUID NOT NULL REFERENCES metric_definitions(id) ON DELETE CASCADE,
    experiment_run_id UUID NOT NULL REFERENCES experiment_runs(id) ON DELETE CASCADE,
    sample_id UUID,
    value DECIMAL(20, 10) NOT NULL,
    metadata JSONB DEFAULT '{}',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT metric_values_unique_sample UNIQUE (metric_definition_id, experiment_run_id, sample_id)
);

-- Apply updated_at trigger
CREATE TRIGGER update_metric_definitions_updated_at BEFORE UPDATE ON metric_definitions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create indexes for metric_definitions
CREATE INDEX idx_metric_definitions_name ON metric_definitions(name);
CREATE INDEX idx_metric_definitions_type ON metric_definitions(metric_type);

-- Create indexes for metric_values
CREATE INDEX idx_metric_values_metric_id ON metric_values(metric_definition_id);
CREATE INDEX idx_metric_values_run_id ON metric_values(experiment_run_id);
CREATE INDEX idx_metric_values_sample_id ON metric_values(sample_id) WHERE sample_id IS NOT NULL;
CREATE INDEX idx_metric_values_recorded_at ON metric_values(recorded_at DESC);
CREATE INDEX idx_metric_values_run_metric ON metric_values(experiment_run_id, metric_definition_id);

-- Create view for aggregated metrics
CREATE OR REPLACE VIEW run_metrics_aggregated AS
SELECT
    mv.experiment_run_id,
    md.name as metric_name,
    md.metric_type,
    md.unit,
    COUNT(*) as sample_count,
    AVG(mv.value) as avg_value,
    MIN(mv.value) as min_value,
    MAX(mv.value) as max_value,
    STDDEV(mv.value) as stddev_value,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY mv.value) as median_value,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY mv.value) as p95_value,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY mv.value) as p99_value
FROM metric_values mv
JOIN metric_definitions md ON mv.metric_definition_id = md.id
GROUP BY mv.experiment_run_id, md.name, md.metric_type, md.unit;
