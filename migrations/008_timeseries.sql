-- Peilbeheer HHVR: Time Series Storage
-- Multi-resolution time series data with automatic downsampling

-- Time series metadata catalog
CREATE TABLE IF NOT EXISTS timeseries_catalog (
    -- Composite key
    location_id VARCHAR NOT NULL,
    parameter VARCHAR NOT NULL,
    qualifier VARCHAR,

    -- Display information
    display_name VARCHAR NOT NULL,
    description TEXT,
    units VARCHAR,

    -- Data characteristics
    data_type VARCHAR NOT NULL DEFAULT 'instantaneous', -- instantaneous, accumulated, average, total, boolean, enum
    source VARCHAR NOT NULL,
    source_type VARCHAR NOT NULL,

    -- Value constraints
    min_value DOUBLE,
    max_value DOUBLE,

    -- Retention policy
    retention_days INTEGER,

    -- Timestamps
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),

    -- Statistics
    first_timestamp TIMESTAMP,
    last_timestamp TIMESTAMP,
    point_count BIGINT DEFAULT 0,

    -- Custom attributes (JSON)
    attributes JSON,

    PRIMARY KEY (location_id, parameter, qualifier)
);

-- Indexes for catalog lookups
CREATE INDEX IF NOT EXISTS idx_timeseries_catalog_source ON timeseries_catalog(source);
CREATE INDEX IF NOT EXISTS idx_timeseries_catalog_source_type ON timeseries_catalog(source_type);
CREATE INDEX IF NOT EXISTS idx_timeseries_catalog_data_type ON timeseries_catalog(data_type);

-- Raw time series data (highest resolution)
CREATE TABLE IF NOT EXISTS timeseries_data_raw (
    series_id VARCHAR NOT NULL, -- location_id|parameter or location_id|parameter|qualifier
    timestamp TIMESTAMP NOT NULL,
    value DOUBLE NOT NULL,
    quality VARCHAR DEFAULT 'good', -- good, questionable, bad, missing, interpolated

    -- Metadata for this point
    source_batch_id VARCHAR,
    attributes JSON,

    PRIMARY KEY (series_id, timestamp)
);

-- Index for time-range queries
CREATE INDEX IF NOT EXISTS idx_timeseries_raw_timestamp ON timeseries_data_raw(series_id, timestamp);

-- 1-minute aggregated data
CREATE TABLE IF NOT EXISTS timeseries_data_1m (
    series_id VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL, -- Truncated to minute

    -- Aggregated values
    avg_value DOUBLE,
    min_value DOUBLE,
    max_value DOUBLE,
    sum_value DOUBLE,
    count INTEGER,

    -- Value at start/end of period
    first_value DOUBLE,
    last_value DOUBLE,

    -- Quality summary
    good_count INTEGER DEFAULT 0,
    bad_count INTEGER DEFAULT 0,
    missing_count INTEGER DEFAULT 0,

    PRIMARY KEY (series_id, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_timeseries_1m_timestamp ON timeseries_data_1m(series_id, timestamp);

-- 5-minute aggregated data
CREATE TABLE IF NOT EXISTS timeseries_data_5m (
    series_id VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL,

    avg_value DOUBLE,
    min_value DOUBLE,
    max_value DOUBLE,
    sum_value DOUBLE,
    count INTEGER,

    first_value DOUBLE,
    last_value DOUBLE,

    good_count INTEGER DEFAULT 0,
    bad_count INTEGER DEFAULT 0,
    missing_count INTEGER DEFAULT 0,

    PRIMARY KEY (series_id, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_timeseries_5m_timestamp ON timeseries_data_5m(series_id, timestamp);

-- 15-minute aggregated data
CREATE TABLE IF NOT EXISTS timeseries_data_15m (
    series_id VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL,

    avg_value DOUBLE,
    min_value DOUBLE,
    max_value DOUBLE,
    sum_value DOUBLE,
    count INTEGER,

    first_value DOUBLE,
    last_value DOUBLE,

    good_count INTEGER DEFAULT 0,
    bad_count INTEGER DEFAULT 0,
    missing_count INTEGER DEFAULT 0,

    PRIMARY KEY (series_id, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_timeseries_15m_timestamp ON timeseries_data_15m(series_id, timestamp);

-- 1-hour aggregated data
CREATE TABLE IF NOT EXISTS timeseries_data_1h (
    series_id VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL,

    avg_value DOUBLE,
    min_value DOUBLE,
    max_value DOUBLE,
    sum_value DOUBLE,
    count INTEGER,

    first_value DOUBLE,
    last_value DOUBLE,

    good_count INTEGER DEFAULT 0,
    bad_count INTEGER DEFAULT 0,
    missing_count INTEGER DEFAULT 0,

    PRIMARY KEY (series_id, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_timeseries_1h_timestamp ON timeseries_data_1h(series_id, timestamp);

-- 1-day aggregated data
CREATE TABLE IF NOT EXISTS timeseries_data_1d (
    series_id VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL, -- Date (time set to 00:00:00)

    avg_value DOUBLE,
    min_value DOUBLE,
    max_value DOUBLE,
    sum_value DOUBLE,
    count INTEGER,

    first_value DOUBLE,
    last_value DOUBLE,

    good_count INTEGER DEFAULT 0,
    bad_count INTEGER DEFAULT 0,
    missing_count INTEGER DEFAULT 0,

    -- Daily statistics
    min_timestamp TIMESTAMP,
    max_timestamp TIMESTAMP,

    PRIMARY KEY (series_id, timestamp)
);

CREATE INDEX IF NOT EXISTS idx_timeseries_1d_timestamp ON timeseries_data_1d(series_id, timestamp);

-- Downsample queue for processing raw data into aggregated tables
CREATE TABLE IF NOT EXISTS timeseries_downsample_queue (
    id VARCHAR PRIMARY KEY,
    series_id VARCHAR NOT NULL,
    level VARCHAR NOT NULL, -- 1m, 5m, 15m, 1h, 1d
    start_timestamp TIMESTAMP NOT NULL,
    end_timestamp TIMESTAMP NOT NULL,

    status VARCHAR DEFAULT 'pending', -- pending, processing, completed, failed
    priority INTEGER DEFAULT 0,

    created_at TIMESTAMP DEFAULT NOW(),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    error_message TEXT,

    -- Retry tracking
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3
);

CREATE INDEX IF NOT EXISTS idx_downsample_queue_status ON timeseries_downsample_queue(status, priority, created_at);
CREATE INDEX IF NOT EXISTS idx_downsample_queue_series ON timeseries_downsample_queue(series_id);

-- Time series gaps tracking
CREATE TABLE IF NOT EXISTS timeseries_gaps (
    id VARCHAR PRIMARY KEY,
    series_id VARCHAR NOT NULL,
    start_timestamp TIMESTAMP NOT NULL,
    end_timestamp TIMESTAMP NOT NULL,
    duration_seconds INTEGER NOT NULL,

    detected_at TIMESTAMP DEFAULT NOW(),
    resolved_at TIMESTAMP,
    status VARCHAR DEFAULT 'open', -- open, resolved, ignored

    expected_points INTEGER,
    actual_points INTEGER,

    -- Resolution tracking
    fill_method VARCHAR, -- linear, forward, backward, constant
    fill_value DOUBLE,

    metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_timeseries_gaps_series ON timeseries_gaps(series_id, status);
CREATE INDEX IF NOT EXISTS idx_timeseries_gaps_detected ON timeseries_gaps(detected_at);

-- Batch write tracking
CREATE TABLE IF NOT EXISTS timeseries_batch_log (
    id VARCHAR PRIMARY KEY,
    series_id VARCHAR NOT NULL,

    source VARCHAR,
    source_type VARCHAR,
    batch_tag VARCHAR,

    -- Write statistics
    points_written INTEGER DEFAULT 0,
    points_updated INTEGER DEFAULT 0,
    points_rejected INTEGER DEFAULT 0,

    -- Time range
    first_timestamp TIMESTAMP,
    last_timestamp TIMESTAMP,

    status VARCHAR DEFAULT 'completed', -- pending, completed, failed
    created_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP,

    error_message TEXT,

    -- Trigger downsampling
    downsample_triggered BOOLEAN DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_batch_log_series ON timeseries_batch_log(series_id, created_at);
CREATE INDEX IF NOT EXISTS idx_batch_log_tag ON timeseries_batch_log(batch_tag);
CREATE INDEX IF NOT EXISTS idx_batch_log_downsample ON timeseries_batch_log(downsample_triggered, status)
    WHERE downsample_triggered = FALSE AND status = 'completed';

-- View for combined time series info
CREATE OR REPLACE VIEW timeseries_info AS
SELECT
    c.location_id || '|' || c.parameter || COALESCE('|' || c.qualifier, '') as series_id,
    c.location_id,
    c.parameter,
    c.qualifier,
    c.display_name,
    c.units,
    c.source,
    c.data_type,
    c.first_timestamp,
    c.last_timestamp,
    c.point_count,
    c.updated_at
FROM timeseries_catalog c;

-- Helper function to truncate timestamp to interval
CREATE OR REPLACE FUNCTION truncate_timestamp(ts TIMESTAMP, interval_val VARCHAR)
RETURNS TIMESTAMP AS $$
BEGIN
    RETURN CASE interval_val
        WHEN '1m' THEN date_trunc('minute', ts)
        WHEN '5m' THEN date_trunc('hour', ts) + CAST(date_part('minute', ts) AS INTEGER) / 5 * INTERVAL '5 minute'
        WHEN '15m' THEN date_trunc('hour', ts) + CAST(date_part('minute', ts) AS INTEGER) / 15 * INTERVAL '15 minute'
        WHEN '1h' THEN date_trunc('hour', ts)
        WHEN '1d' THEN date_trunc('day', ts)
        ELSE ts
    END;
END;
$$ LANGUAGE plpgsql;
