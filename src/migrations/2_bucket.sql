CREATE TABLE public.bucket_facts (
    id serial PRIMARY KEY,
    fact text NOT NULL,
    verb text DEFAULT 'is'::text NOT NULL,
    tidbit text NOT NULL,
    UNIQUE(fact, verb, tidbit)
);
