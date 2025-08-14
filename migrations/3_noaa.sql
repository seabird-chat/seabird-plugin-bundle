CREATE TABLE IF NOT EXISTS noaa_location (
    id serial PRIMARY KEY,
    nick text UNIQUE NOT NULL,
    station text NOT NULL
);
