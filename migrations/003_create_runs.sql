-- Create experiment run status enum
CREATE TYPE run_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');

-- Create experiment_runs table
CREATE TABLE IF NOT EXISTS experiment_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    run_number INTEGER NOT NULL,
    status run_status NOT NULL DEFAULT 'pending',
    config JSONB NOT NULL DEFAULT '{}',
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    total_samples INTEGER DEFAULT 0,
    successful_samples INTEGER DEFAULT 0,
    failed_samples INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    total_cost DECIMAL(12, 6),
    error_message TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT experiment_runs_run_number_positive CHECK (run_number > 0),
    CONSTRAINT experiment_runs_unique_number UNIQUE (experiment_id, run_number)
);

-- Apply updated_at trigger
CREATE TRIGGER update_experiment_runs_updated_at BEFORE UPDATE ON experiment_runs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create indexes
CREATE INDEX idx_experiment_runs_experiment_id ON experiment_runs(experiment_id);
CREATE INDEX idx_experiment_runs_status ON experiment_runs(status);
CREATE INDEX idx_experiment_runs_created_at ON experiment_runs(created_at DESC);
CREATE INDEX idx_experiment_runs_number ON experiment_runs(experiment_id, run_number DESC);

-- Create function to get next run number
CREATE OR REPLACE FUNCTION get_next_run_number(exp_id UUID)
RETURNS INTEGER AS $$
DECLARE
    next_num INTEGER;
BEGIN
    SELECT COALESCE(MAX(run_number), 0) + 1 INTO next_num
    FROM experiment_runs
    WHERE experiment_id = exp_id;
    RETURN next_num;
END;
$$ LANGUAGE plpgsql;
