-- Create experiment status enum
CREATE TYPE experiment_status AS ENUM ('draft', 'running', 'completed', 'failed', 'cancelled');

-- Create experiments table
CREATE TABLE IF NOT EXISTS experiments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    status experiment_status NOT NULL DEFAULT 'draft',
    owner_id UUID REFERENCES users(id) ON DELETE CASCADE,
    model_id UUID NOT NULL,
    dataset_id UUID NOT NULL,
    prompt_template_id UUID NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    CONSTRAINT experiments_name_length CHECK (char_length(name) > 0 AND char_length(name) <= 255)
);

-- Apply updated_at trigger
CREATE TRIGGER update_experiments_updated_at BEFORE UPDATE ON experiments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create indexes
CREATE INDEX idx_experiments_status ON experiments(status);
CREATE INDEX idx_experiments_owner_id ON experiments(owner_id);
CREATE INDEX idx_experiments_model_id ON experiments(model_id);
CREATE INDEX idx_experiments_dataset_id ON experiments(dataset_id);
CREATE INDEX idx_experiments_prompt_template_id ON experiments(prompt_template_id);
CREATE INDEX idx_experiments_created_at ON experiments(created_at DESC);
CREATE INDEX idx_experiments_tags ON experiments USING GIN(tags);
CREATE INDEX idx_experiments_name_trgm ON experiments USING gin(name gin_trgm_ops);

-- Enable pg_trgm extension for text search
CREATE EXTENSION IF NOT EXISTS pg_trgm;
