-- Peilbeheer HHVR: DuckDB Schema
-- Gebaseerd op peilbesluiten/db_gemaal.py

-- Laatste status per gemaal (snapshot)
CREATE TABLE IF NOT EXISTS gemaal_status_snapshot (
    gemaal_code VARCHAR PRIMARY KEY,
    status VARCHAR NOT NULL DEFAULT 'unknown',
    debiet DOUBLE DEFAULT 0.0,
    last_update TIMESTAMP,
    generated_at TIMESTAMP,
    trends_json VARCHAR
);

-- Gemiddeld debiet per uur (7 dagen historie)
CREATE TABLE IF NOT EXISTS gemaal_debiet_per_uur (
    gemaal_code VARCHAR NOT NULL,
    hour_utc TIMESTAMP NOT NULL,
    avg_debiet DOUBLE DEFAULT 0.0,
    n_metingen INTEGER DEFAULT 0,
    PRIMARY KEY (gemaal_code, hour_utc)
);

-- Gemaal registratie (cached van ArcGIS GeoJSON)
CREATE TABLE IF NOT EXISTS gemaal_registratie (
    code VARCHAR PRIMARY KEY,
    naam VARCHAR,
    latitude DOUBLE,
    longitude DOUBLE,
    capaciteit DOUBLE,
    functie VARCHAR,
    soort VARCHAR,
    plaats VARCHAR,
    gemeente VARCHAR,
    fetched_at TIMESTAMP NOT NULL
)
