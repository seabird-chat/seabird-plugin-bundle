CREATE TABLE IF NOT EXISTS bucket_facts (
    id serial PRIMARY KEY,
    fact text NOT NULL,
    verb text DEFAULT 'is' NOT NULL,
    tidbit text NOT NULL,
    UNIQUE(fact, verb, tidbit)
);
