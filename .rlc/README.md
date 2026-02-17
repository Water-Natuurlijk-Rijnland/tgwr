# RLC Setup

This directory contains your Runtime LifeCycle configuration.

## Quick Start

### Start Observability Stack (Budget Tier)

```bash
docker-compose -f docker-compose.obs.yml up -d
```

### Verify Configuration

```bash
./.rlc/scripts/verify-config.sh
```

### Test Event Ingestion

```bash
./.rlc/scripts/test-ingestion.sh
```

## Configuration

### Event Handling
- **Tier**: balanced
- **Metrics**: N/A
- **Logs**: N/A
- **Traces**: N/A
- **Alerts**: N/A

### Environment
- **Platform**: paas_vercel
- **Provider**: vercel
- **Languages**: javascript, rust, dotnet

## Agents

The following RLC agents are configured:

- **incident-commander**: Core agent
- **auto-remediator**: Core agent
- **post-mortem-writer**: Core agent


## Next Steps

1. Configure your credentials in the appropriate `.env.*` files
2. Start the observability stack
3. Run the verification scripts
4. Deploy your application with instrumentation

## Documentation

See the main RLC documentation for more details on agent capabilities.
