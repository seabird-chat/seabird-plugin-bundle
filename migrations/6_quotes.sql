CREATE TABLE IF NOT EXISTS quotes (
    nick text NOT NULL,
    quote text NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_quotes_nick ON quotes(nick);
