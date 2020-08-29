CREATE TABLE IF NOT EXISTS public.forecast_location (
    id serial PRIMARY KEY,
    nick text UNIQUE NOT NULL,
    address text NOT NULL,
    lat double precision NOT NULL,
    lng double precision NOT NULL
);
