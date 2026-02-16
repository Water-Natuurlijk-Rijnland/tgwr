-- Peilbeheer HHVR: Alert Rule Engine
-- Tables for alert rules, triggered alerts, and notification history

-- Alert rules table
CREATE TABLE IF NOT EXISTS alert_rules (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,

    -- Rule classification
    category VARCHAR NOT NULL DEFAULT 'custom', -- water_level, pump_status, energy_price, weather, system_health, simulation, custom
    severity VARCHAR NOT NULL DEFAULT 'warning', -- info, warning, error, critical

    -- Rule conditions (JSON array)
    conditions JSON NOT NULL, -- [{"field": "water_level", "operator": "gte", "value": 2.5}]
    condition_logic VARCHAR NOT NULL DEFAULT 'and', -- and, or

    -- Cooldown period to prevent alert spam
    cooldown_seconds INTEGER DEFAULT 300, -- 5 minutes default

    -- Enable/disable
    enabled BOOLEAN DEFAULT TRUE,

    -- Notification channels (JSON array)
    notification_channels JSON DEFAULT '[{"WebSocket": []}]', -- WebSocket, Email, Webhook, Sms

    -- Alert templates
    title_template VARCHAR NOT NULL DEFAULT '{category} Alert: {rule_name}',
    message_template TEXT,

    -- Metadata (JSON)
    metadata JSON,

    -- Audit fields
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR
);

-- Indexes for alert rules
CREATE INDEX IF NOT EXISTS idx_alert_rules_category ON alert_rules(category);
CREATE INDEX IF NOT EXISTS idx_alert_rules_severity ON alert_rules(severity);
CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled ON alert_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_alert_rules_created_by ON alert_rules(created_by);

-- Triggered alerts table
CREATE TABLE IF NOT EXISTS alerts (
    id VARCHAR PRIMARY KEY,
    rule_id VARCHAR NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    rule_name VARCHAR NOT NULL,

    -- Alert classification (copied from rule at trigger time)
    category VARCHAR NOT NULL,
    severity VARCHAR NOT NULL,

    -- Alert content
    title VARCHAR NOT NULL,
    message TEXT,

    -- Affected resources (JSON array of IDs)
    affected_resources JSON, -- ["GEMAAL_001", "GEMAAL_002"]

    -- Alert lifecycle
    status VARCHAR NOT NULL DEFAULT 'active', -- active, acknowledged, resolved, suppressed
    triggered_at TIMESTAMP NOT NULL,
    acknowledged_at TIMESTAMP,
    acknowledged_by VARCHAR,
    resolved_at TIMESTAMP,

    -- Context data at trigger time (JSON)
    context JSON,

    -- Optional metadata
    metadata JSON
);

-- Indexes for alerts
CREATE INDEX IF NOT EXISTS idx_alerts_rule_id ON alerts(rule_id);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
CREATE INDEX IF NOT EXISTS idx_alerts_category ON alerts(category);
CREATE INDEX IF NOT EXISTS idx_alerts_triggered_at ON alerts(triggered_at);
CREATE INDEX IF NOT EXISTS idx_alerts_acknowledged_by ON alerts(acknowledged_by);
CREATE INDEX IF NOT EXISTS idx_alerts_active_severity ON alerts(status, severity) WHERE status = 'active';

-- Alert history for analysis
CREATE TABLE IF NOT EXISTS alert_history (
    id VARCHAR PRIMARY KEY,
    alert_id VARCHAR NOT NULL REFERENCES alerts(id) ON DELETE CASCADE,
    event_type VARCHAR NOT NULL, -- triggered, acknowledged, resolved, suppressed, notification_sent
    event_data JSON,

    -- Who/what caused the event
    caused_by VARCHAR,
    cause_type VARCHAR, -- user, system, rule_engine

    -- Timestamp
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alert_history_alert_id ON alert_history(alert_id);
CREATE INDEX IF NOT EXISTS idx_alert_history_event_type ON alert_history(event_type);
CREATE INDEX IF NOT EXISTS idx_alert_history_created_at ON alert_history(created_at);

-- Notification log
CREATE TABLE IF NOT EXISTS alert_notifications (
    id VARCHAR PRIMARY KEY,
    alert_id VARCHAR NOT NULL REFERENCES alerts(id) ON DELETE CASCADE,

    -- Channel details
    channel_type VARCHAR NOT NULL, -- WebSocket, Email, Webhook, Sms
    channel_target VARCHAR, -- email address, webhook URL, phone number

    -- Delivery status
    status VARCHAR NOT NULL DEFAULT 'pending', -- pending, sent, failed, retrying
    sent_at TIMESTAMP,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,

    -- Request/response details
    request_payload JSON,
    response_payload JSON,

    -- Timestamp
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alert_notifications_alert_id ON alert_notifications(alert_id);
CREATE INDEX IF NOT EXISTS idx_alert_notifications_status ON alert_notifications(status);
CREATE INDEX IF NOT EXISTS idx_alert_notifications_channel_type ON alert_notifications(channel_type);
CREATE INDEX IF NOT EXISTS idx_alert_notifications_created_at ON alert_notifications(created_at);
