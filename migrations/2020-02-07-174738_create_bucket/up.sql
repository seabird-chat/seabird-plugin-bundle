CREATE TABLE bucket_facts (
    id SERIAL NOT NULL PRIMARY KEY,
    fact TEXT NOT NULL,
    verb TEXT NOT NULL DEFAULT 'is',
    tidbit TEXT NOT NULL,

    UNIQUE (fact, verb, tidbit)
);

CREATE INDEX bucket_fact_lookup ON bucket_facts (fact);
