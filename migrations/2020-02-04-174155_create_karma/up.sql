CREATE TABLE karma (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    score INTEGER DEFAULT 0 NOT NULL
);
CREATE UNIQUE INDEX karma_name ON karma(name);
