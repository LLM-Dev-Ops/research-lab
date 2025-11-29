-- Create audit_log table for storing audit events
CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    event_type JSONB NOT NULL,
    actor JSONB NOT NULL,
    resource JSONB NOT NULL,
    action JSONB NOT NULL,
    outcome JSONB NOT NULL,
    details JSONB,
    ip_address TEXT,
    user_agent TEXT,
    request_id TEXT,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for common queries
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_log_event_type ON audit_log((event_type->>'type'));
CREATE INDEX idx_audit_log_actor ON audit_log((actor->>'type'));
CREATE INDEX idx_audit_log_resource ON audit_log((resource->>'type'));
CREATE INDEX idx_audit_log_action ON audit_log((action));
CREATE INDEX idx_audit_log_outcome ON audit_log((outcome->>'status'));
CREATE INDEX idx_audit_log_ip_address ON audit_log(ip_address);
CREATE INDEX idx_audit_log_request_id ON audit_log(request_id);

-- Create a composite index for common filtering scenarios
CREATE INDEX idx_audit_log_type_timestamp ON audit_log((event_type->>'type'), timestamp DESC);

-- Create a GIN index for full JSON queries if needed
CREATE INDEX idx_audit_log_details_gin ON audit_log USING GIN(details);

-- Enable row-level security (optional, can be enabled later)
-- ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;

-- Add comments for documentation
COMMENT ON TABLE audit_log IS 'Stores comprehensive audit trail of all system operations';
COMMENT ON COLUMN audit_log.id IS 'Unique identifier for the audit event';
COMMENT ON COLUMN audit_log.timestamp IS 'When the audited event occurred';
COMMENT ON COLUMN audit_log.event_type IS 'Type of event (authentication, authorization, data_access, etc.)';
COMMENT ON COLUMN audit_log.actor IS 'Who performed the action (user, api_key, system, anonymous)';
COMMENT ON COLUMN audit_log.resource IS 'What resource was affected';
COMMENT ON COLUMN audit_log.action IS 'What action was performed';
COMMENT ON COLUMN audit_log.outcome IS 'Result of the operation (success, failure, denied)';
COMMENT ON COLUMN audit_log.details IS 'Additional structured details about the event';
COMMENT ON COLUMN audit_log.ip_address IS 'IP address of the requester';
COMMENT ON COLUMN audit_log.user_agent IS 'User agent string from the request';
COMMENT ON COLUMN audit_log.request_id IS 'Request ID for correlation with application logs';
COMMENT ON COLUMN audit_log.duration_ms IS 'How long the operation took in milliseconds';
