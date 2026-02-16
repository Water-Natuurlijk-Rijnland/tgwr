-- Peilbeheer HHVR: Scenario Resultaten
-- Opslag van DHYdro scenario uitvoeringsresultaten

-- Scenario uitvoeringsresultaten
CREATE TABLE IF NOT EXISTS scenario_results (
    id VARCHAR PRIMARY KEY,
    scenario_id VARCHAR NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,

    -- Uitvoeringsstatus
    status VARCHAR NOT NULL, -- pending, running, completed, failed, cancelled
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    duration_seconds INTEGER,

    -- Resultaat data (JSON voor flexibiliteit)
    results_summary JSON,
    time_series_count INTEGER DEFAULT 0,
    output_files JSON, -- ['path/to/output1.nc', 'path/to/output2.png']

    -- Foutinformatie
    error_message TEXT,
    error_code VARCHAR,

    -- Metadaten
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR,

    -- DHYdro specifiek
    dhydro_job_id VARCHAR, -- Externe job ID van DHYdro API
    dhydro_result_url VARCHAR -- URL naar resultaten op DHYdro server
);

CREATE INDEX IF NOT EXISTS idx_scenario_results_scenario_id ON scenario_results(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenario_results_status ON scenario_results(status);
CREATE INDEX IF NOT EXISTS idx_scenario_results_started_at ON scenario_results(started_at);
CREATE INDEX IF NOT EXISTS idx_scenario_results_dhydro_job ON scenario_results(dhydro_job_id) WHERE dhydro_job_id IS NOT NULL;

-- Time series resultaten (individuele reeksen per locatie/parameter)
CREATE TABLE IF NOT EXISTS scenario_result_timeseries (
    id VARCHAR PRIMARY KEY,
    result_id VARCHAR NOT NULL REFERENCES scenario_results(id) ON DELETE CASCADE,

    -- Tijdreeks metadata
    location_id VARCHAR NOT NULL,
    location_name VARCHAR,
    parameter VARCHAR NOT NULL, -- water_level, discharge, volume, etc.
    unit VARCHAR,

    -- Data (opgeslagen als JSON array of binair)
    -- Formaat: [{"timestamp": "...", "value": 1.5, "flag": "valid"}, ...]
    data JSON NOT NULL,

    -- Statistische samenvatting
    min_value DOUBLE,
    max_value DOUBLE,
    avg_value DOUBLE,
    first_value DOUBLE,
    last_value DOUBLE,

    -- Ruwe data opslag (optioneel, voor grote datasets)
    raw_data_path VARCHAR, -- Pad naar NetCDF of andere bestanden

    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scenario_result_ts_result_id ON scenario_result_timeseries(result_id);
CREATE INDEX IF NOT EXISTS idx_scenario_result_ts_location ON scenario_result_timeseries(location_id);
CREATE INDEX IF NOT EXISTS idx_scenario_result_ts_parameter ON scenario_result_timeseries(parameter);

-- Scenario vergelijkingstabel (voor vergelijken van scenario's)
CREATE TABLE IF NOT EXISTS scenario_comparisons (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR
);

CREATE TABLE IF NOT EXISTS scenario_comparison_items (
    id VARCHAR PRIMARY KEY,
    comparison_id VARCHAR NOT NULL REFERENCES scenario_comparisons(id) ON DELETE CASCADE,
    scenario_id VARCHAR NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    display_name VARCHAR NOT NULL,
    color VARCHAR, -- Hex kleur voor visualisatie
    is_baseline BOOLEAN DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_scenario_comparison_items_comparison ON scenario_comparison_items(comparison_id);
CREATE INDEX IF NOT EXISTS idx_scenario_comparison_items_scenario ON scenario_comparison_items(scenario_id);
