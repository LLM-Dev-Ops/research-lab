-- Create artifact type enum
CREATE TYPE artifact_type AS ENUM ('model_output', 'intermediate_result', 'log', 'visualization', 'report', 'other');

-- Create artifacts table
CREATE TABLE IF NOT EXISTS artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    experiment_run_id UUID NOT NULL REFERENCES experiment_runs(id) ON DELETE CASCADE,
    artifact_type artifact_type NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    s3_path VARCHAR(1024) NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    size_bytes BIGINT NOT NULL,
    mime_type VARCHAR(127),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT artifacts_name_length CHECK (char_length(name) > 0 AND char_length(name) <= 255),
    CONSTRAINT artifacts_size_nonnegative CHECK (size_bytes >= 0)
);

-- Create models table
CREATE TABLE IF NOT EXISTS models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    model_identifier VARCHAR(255) NOT NULL,
    version VARCHAR(50),
    config JSONB NOT NULL DEFAULT '{}',
    owner_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT models_name_length CHECK (char_length(name) > 0 AND char_length(name) <= 255),
    CONSTRAINT models_identifier_length CHECK (char_length(model_identifier) > 0 AND char_length(model_identifier) <= 255)
);

-- Create prompt_templates table
CREATE TABLE IF NOT EXISTS prompt_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    template TEXT NOT NULL,
    variables TEXT[] DEFAULT ARRAY[]::TEXT[],
    version INTEGER NOT NULL DEFAULT 1,
    owner_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT prompt_templates_name_length CHECK (char_length(name) > 0 AND char_length(name) <= 255),
    CONSTRAINT prompt_templates_template_nonempty CHECK (char_length(template) > 0),
    CONSTRAINT prompt_templates_version_positive CHECK (version > 0)
);

-- Apply updated_at triggers
CREATE TRIGGER update_models_updated_at BEFORE UPDATE ON models
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_prompt_templates_updated_at BEFORE UPDATE ON prompt_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create indexes for artifacts
CREATE INDEX idx_artifacts_run_id ON artifacts(experiment_run_id);
CREATE INDEX idx_artifacts_type ON artifacts(artifact_type);
CREATE INDEX idx_artifacts_created_at ON artifacts(created_at DESC);
CREATE INDEX idx_artifacts_content_hash ON artifacts(content_hash);

-- Create indexes for models
CREATE INDEX idx_models_provider ON models(provider);
CREATE INDEX idx_models_owner_id ON models(owner_id);
CREATE INDEX idx_models_created_at ON models(created_at DESC);
CREATE INDEX idx_models_name_trgm ON models USING gin(name gin_trgm_ops);

-- Create indexes for prompt_templates
CREATE INDEX idx_prompt_templates_owner_id ON prompt_templates(owner_id);
CREATE INDEX idx_prompt_templates_created_at ON prompt_templates(created_at DESC);
CREATE INDEX idx_prompt_templates_version ON prompt_templates(version DESC);
CREATE INDEX idx_prompt_templates_name_trgm ON prompt_templates USING gin(name gin_trgm_ops);
