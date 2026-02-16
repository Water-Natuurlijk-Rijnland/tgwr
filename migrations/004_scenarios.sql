-- Peilbeheer HHVR: Scenario Management
-- Opslag van DHYdro scenario's en parameters

-- Scenario tabel voor opslag van hydraulische model scenario's
CREATE TABLE IF NOT EXISTS scenarios (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,
    model_id VARCHAR NOT NULL,
    model_type VARCHAR,

    -- Scenario parameters (JSON voor flexibiliteit)
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
    time_step INTEGER NOT NULL, -- in seconden

    -- Boundary conditions (JSON: inlet/outlet levels, flows, etc.)
    boundary_conditions JSON,

    -- Initial conditions (JSON: start water levels, etc.)
    initial_conditions JSON,

    -- Model-specific parameters (JSON: roughness, weirs, etc.)
    model_parameters JSON,

    -- Metadata
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR,
    updated_at TIMESTAMP DEFAULT NOW(),

    -- Scenario relatie
    is_base_scenario BOOLEAN DEFAULT FALSE,
    base_scenario_id VARCHAR REFERENCES scenarios(id),

    -- Status
    status VARCHAR DEFAULT 'draft', -- draft, active, archived
    tags JSON -- '["flood", "extreme"]'
);

-- Indexen voor vaakgebruikte queries
CREATE INDEX IF NOT EXISTS idx_scenarios_model_id ON scenarios(model_id);
CREATE INDEX IF NOT EXISTS idx_scenarios_status ON scenarios(status);
CREATE INDEX IF NOT EXISTS idx_scenarios_created_by ON scenarios(created_by);
CREATE INDEX IF NOT EXISTS idx_scenarios_base_scenario ON scenarios(base_scenario_id) WHERE base_scenario_id IS NOT NULL;

-- Full-text search op naam en beschrijving
-- DuckDB ondersteunt geen full-text, dus we gebruiken een simpele LIKE index
CREATE INDEX IF NOT EXISTS idx_scenarios_name_trgm ON scenarios USING fts_stemedge(id, name, description);
-- Als full-text niet beschikbaar is, gebruik gewone index:
-- CREATE INDEX IF NOT EXISTS idx_scenarios_name ON scenarios(name);

-- Scenario versiegeschiedenis (voor audit trailing)
CREATE TABLE IF NOT EXISTS scenarios_history (
    id VARCHAR PRIMARY KEY,
    scenario_id VARCHAR NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    changed_at TIMESTAMP DEFAULT NOW(),
    changed_by VARCHAR,
    change_type VARCHAR NOT NULL, -- created, updated, deleted, executed
    old_values JSON,
    new_values JSON
);

CREATE INDEX IF NOT EXISTS idx_scenarios_history_scenario_id ON scenarios_history(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenarios_history_changed_at ON scenarios_history(changed_at);
