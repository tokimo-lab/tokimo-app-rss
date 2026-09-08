CREATE TABLE sources (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL,
    name varchar(200) NOT NULL,
    url text NOT NULL,
    normalized_url text NOT NULL,
    enabled boolean NOT NULL DEFAULT true,
    archived_at timestamptz,
    poll_interval_seconds integer NOT NULL DEFAULT 300 CHECK (poll_interval_seconds >= 60),
    etag varchar(1000),
    last_modified varchar(1000),
    last_polled_at timestamptz,
    last_success_at timestamptz,
    next_poll_at timestamptz NOT NULL DEFAULT now(),
    initialized_at timestamptz,
    last_window_ids text[] NOT NULL DEFAULT '{}',
    possible_gap boolean NOT NULL DEFAULT false,
    failure_count integer NOT NULL DEFAULT 0,
    last_error text,
    lease_token uuid,
    lease_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (user_id, normalized_url)
);

CREATE TABLE entries (
    id uuid PRIMARY KEY,
    source_id uuid NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    external_id text NOT NULL,
    url text NOT NULL,
    title varchar(1000) NOT NULL,
    summary text,
    categories text[] NOT NULL DEFAULT '{}',
    author varchar(500),
    published_at timestamptz,
    first_seen_at timestamptz NOT NULL,
    last_seen_at timestamptz NOT NULL,
    sort_at timestamptz GENERATED ALWAYS AS (COALESCE(published_at, first_seen_at)) STORED,
    normalized_title text NOT NULL,
    normalized_summary text,
    UNIQUE (source_id, external_id)
);

CREATE TABLE rules (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL,
    source_id uuid NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    name varchar(200) NOT NULL,
    enabled boolean NOT NULL DEFAULT true,
    categories text[] NOT NULL DEFAULT '{}',
    include_any text[] NOT NULL DEFAULT '{}',
    exclude_any text[] NOT NULL DEFAULT '{}',
    match_scope varchar(32) NOT NULL DEFAULT 'title' CHECK (match_scope IN ('title', 'title_summary')),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL
);

CREATE TABLE deliveries (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL,
    entry_id uuid NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    matched_rules uuid[] NOT NULL,
    matched_rule_names text[] NOT NULL,
    status varchar(32) NOT NULL CHECK (status IN ('pending', 'submitting', 'accepted', 'cancelled')),
    attempts integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL,
    last_error text,
    accepted_at timestamptz,
    lease_token uuid,
    lease_until timestamptz,
    dedupe_key text NOT NULL,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    UNIQUE (user_id, entry_id),
    UNIQUE (dedupe_key)
);

CREATE INDEX sources_due_idx ON sources (next_poll_at) WHERE enabled AND archived_at IS NULL;
CREATE INDEX sources_user_idx ON sources (user_id, archived_at, created_at);
CREATE INDEX entries_source_time_idx ON entries (source_id, sort_at DESC, id DESC);
CREATE INDEX entries_seen_idx ON entries (source_id, first_seen_at DESC, id DESC);
CREATE INDEX rules_source_user_idx ON rules (source_id, user_id) WHERE enabled;
CREATE INDEX deliveries_due_idx ON deliveries (next_attempt_at) WHERE status IN ('pending', 'submitting');
CREATE INDEX deliveries_user_idx ON deliveries (user_id, created_at DESC);
