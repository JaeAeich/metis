ALTER TABLE runs ADD COLUMN deleted_at TIMESTAMPTZ;
CREATE INDEX idx_runs_deleted_at ON runs (deleted_at) WHERE deleted_at IS NOT NULL;
