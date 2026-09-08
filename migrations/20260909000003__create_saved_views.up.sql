CREATE TABLE rss.saved_views (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL,
    source_id uuid NOT NULL REFERENCES rss.sources(id) ON DELETE CASCADE,
    name varchar(200) NOT NULL,
    categories text[] NOT NULL DEFAULT '{}',
    include_any text[] NOT NULL DEFAULT '{}',
    exclude_any text[] NOT NULL DEFAULT '{}',
    match_scope varchar(32) NOT NULL DEFAULT 'title',
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    CONSTRAINT saved_views_match_scope_check CHECK (match_scope IN ('title', 'title_summary')),
    CONSTRAINT saved_views_user_source_name_key UNIQUE (user_id, source_id, name)
);

CREATE INDEX saved_views_user_source_created_idx
    ON rss.saved_views(user_id, source_id, created_at);
