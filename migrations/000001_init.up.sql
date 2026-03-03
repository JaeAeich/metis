CREATE TABLE runs (
    run_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'UNKNOWN',
    workflow_type TEXT NOT NULL,
    workflow_type_version TEXT NOT NULL,
    workflow_url TEXT NOT NULL,
    workflow_engine TEXT,
    workflow_engine_version TEXT,
    workflow_params JSONB,
    workflow_engine_parameters JSONB,
    tags JSONB NOT NULL DEFAULT '{}',
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE run_logs (
    id BIGSERIAL PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs (run_id),
    name TEXT,
    cmd JSONB,
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    stdout TEXT,
    stderr TEXT,
    exit_code INT,
    system_logs JSONB
);

CREATE TABLE log_lines (
    id BIGSERIAL PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs (run_id),
    stream TEXT NOT NULL CHECK (stream IN ('stdout', 'stderr')),
    line TEXT NOT NULL,
    seq BIGINT NOT NULL,
    written_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE task_logs (
    id BIGSERIAL PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs (run_id),
    task_id TEXT NOT NULL,
    name TEXT NOT NULL,
    cmd JSONB,
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    stdout TEXT,
    stderr TEXT,
    exit_code INT,
    system_logs JSONB,
    tes_uri TEXT
);

-- Indexes for ListRuns filtering
CREATE INDEX idx_runs_state ON runs (state);
CREATE INDEX idx_runs_user_id ON runs (user_id);
CREATE INDEX idx_runs_start_time ON runs (start_time);

-- Index for log line streaming/pagination
CREATE INDEX idx_log_lines_run_stream_seq ON log_lines (run_id, stream, seq);
