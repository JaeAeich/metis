DROP INDEX IF EXISTS idx_runs_deleted_at;
ALTER TABLE runs DROP COLUMN deleted_at;
