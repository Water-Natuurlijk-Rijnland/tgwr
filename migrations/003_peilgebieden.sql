-- Peilgebieden: polygonen uit GeoJSON met spatial geometry
CREATE TABLE IF NOT EXISTS peilgebied (
    code VARCHAR PRIMARY KEY,
    naam VARCHAR,
    zomerpeil DOUBLE,
    winterpeil DOUBLE,
    vastpeil DOUBLE,
    oppervlakte DOUBLE,
    soortafwatering VARCHAR,
    soortpeilgebied VARCHAR,
    geometry GEOMETRY
)
