# RLC Setup for VERCEL PAAS_VERCEL

## Environment Analysis

- **Languages**: javascript, rust, dotnet
- **Frameworks**: None detected
- **Compute Platform**: paas_vercel
- **Cloud Provider**: vercel

## Event Handling Options

**Selected Option**: Vercel + Grafana Cloud (BALANCED)

### All Options (Ranked by Price-Quality Ratio)

### Option 1: Vercel Analytics + Log Drains (BUDGET) 

- **Estimated Cost**: <$20
- **Setup Complexity**: High
- **Metrics**: Vercel Analytics (free tier)
- **Logs**: Vercel Logs → self-hosted Loki
- **Traces**: OpenTelemetry (self-instrumented)
- **Alerts**: Vercel Notifications

**Pros**:
  - Free Vercel Analytics
  - Control logs
  - Edge metrics included

**Cons**:
  - Self-hosted Loki
  - Limited traces

**Best For**: Vercel hobby projects, Budget constrained


### Option 2: Vercel + Grafana Cloud (BALANCED) ⭐ RECOMMENDED

- **Estimated Cost**: $20-60
- **Setup Complexity**: Medium
- **Metrics**: Vercel Analytics + Grafana Cloud
- **Logs**: Grafana Cloud Loki (via drain)
- **Traces**: Grafana Cloud Tempo
- **Alerts**: Grafana OnCall

**Pros**:
  - Excellent edge monitoring
  - Unified dashboard
  - Reasonable cost

**Cons**:
  - Two platforms

**Best For**: Production Vercel apps, E-commerce


### Option 3: Vercel + Datadog (PREMIUM) 

- **Estimated Cost**: $75-300
- **Setup Complexity**: Low
- **Metrics**: Datadog + Vercel Analytics
- **Logs**: Datadog Logs
- **Traces**: Datadog RUM + APM
- **Alerts**: Datadog Monitor

**Pros**:
  - Full RUM + APM
  - Turnkey
  - Excellent for frontend

**Cons**:
  - Expensive

**Best For**: Enterprise Vercel, Customer-facing apps



## Selected Option Details

### Metrics
- **Source**: Vercel Analytics + Grafana Cloud
- **Ingestion**: Log drain

### Logs
- **Source**: Grafana Cloud Loki (via drain)
- **Ingestion**: OTel instrumentation

### Traces
- **Source**: Grafana Cloud Tempo
- **Ingestion**: Log drain

### Alerts
- **Destination**: Grafana OnCall

### Setup Commands
1. Configure log drain in Vercel

## Agent Team Configuration

### Core Agents
- incident-commander
- auto-remediator
- post-mortem-writer

### Observers
- health-checker
- metrics-collector

### Monitors
- edge-function-monitor
- deployment-monitor
- threshold-evaluator

### Alerters
- alert-router

### Controllers
- auto-remediator

### Responders
- recovery-monitor
- runbook-executor

### Optional (Recommended)
- cost-analyzer

## Setup Steps

1. **Install agents**
   ```bash
   ./install-agents.sh
   ```

2. **Configure event handling**
   Follow the setup commands above for your selected option.

3. **Test the setup**
   ```bash
   python events/ingestion/event-ingester.py --type test --severity low --title "Test event"
   ```

## Changing Option

To change the selected event handling option, re-run the wizard with:
```bash
python tools/wizard/rlc-setup-wizard.py <repo-path> --tier [budget|balanced|premium]
```

