-- Asset registratie: generieke tabel voor alle ArcGIS-lagen
CREATE TABLE IF NOT EXISTS asset_registratie (
    layer_type VARCHAR NOT NULL,
    code VARCHAR NOT NULL,
    naam VARCHAR,
    latitude DOUBLE,
    longitude DOUBLE,
    extra_properties VARCHAR,
    fetched_at TIMESTAMP NOT NULL,
    PRIMARY KEY (layer_type, code)
)
