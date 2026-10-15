-- @req RF-34
CREATE TABLE search_history (
    query_key TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    searched_at INTEGER NOT NULL
) STRICT;
CREATE INDEX idx_search_history_recent ON search_history (searched_at DESC);
