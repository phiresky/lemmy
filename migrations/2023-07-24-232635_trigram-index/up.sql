CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX idx_comment_content_trigram ON comment USING gin(content gin_trgm_ops);

CREATE INDEX idx_post_trigram ON post USING gin(name gin_trgm_ops, body gin_trgm_ops);

CREATE INDEX idx_person_trigram ON person USING gin(name gin_trgm_ops, display_name gin_trgm_ops);

