CREATE TABLE IF NOT EXISTS public.karma (
    name text NOT NULL PRIMARY KEY,
    score integer DEFAULT 0 NOT NULL
);
