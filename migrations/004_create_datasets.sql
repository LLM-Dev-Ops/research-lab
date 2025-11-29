-- Create datasets table
CREATE TABLE IF NOT EXISTS datasets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    owner_id UUID REFERENCES users(id) ON DELETE CASCADE,
    s3_path VARCHAR(1024) NOT NULL,
    sample_count BIGINT NOT NULL DEFAULT 0,
    schema JSONB NOT NULL DEFAULT '{}',
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT datasets_name_length CHECK (char_length(name) > 0 AND char_length(name) <= 255),
    CONSTRAINT datasets_sample_count_nonnegative CHECK (sample_count >= 0)
);

-- Create dataset_versions table for versioning
CREATE TABLE IF NOT EXISTS dataset_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    s3_path VARCHAR(1024) NOT NULL,
    sample_count BIGINT NOT NULL DEFAULT 0,
    schema JSONB NOT NULL DEFAULT '{}',
    changes_description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    CONSTRAINT dataset_versions_version_positive CHECK (version > 0),
    CONSTRAINT dataset_versions_unique_version UNIQUE (dataset_id, version)
);

-- Apply updated_at trigger
CREATE TRIGGER update_datasets_updated_at BEFORE UPDATE ON datasets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create indexes for datasets
CREATE INDEX idx_datasets_owner_id ON datasets(owner_id);
CREATE INDEX idx_datasets_created_at ON datasets(created_at DESC);
CREATE INDEX idx_datasets_tags ON datasets USING GIN(tags);
CREATE INDEX idx_datasets_name_trgm ON datasets USING gin(name gin_trgm_ops);

-- Create indexes for dataset_versions
CREATE INDEX idx_dataset_versions_dataset_id ON dataset_versions(dataset_id);
CREATE INDEX idx_dataset_versions_created_at ON dataset_versions(created_at DESC);
CREATE INDEX idx_dataset_versions_version ON dataset_versions(dataset_id, version DESC);
