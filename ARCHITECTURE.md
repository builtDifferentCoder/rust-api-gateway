# Architecture

```mermaid
flowchart LR
  Client -->|HTTP| Gateway[API Gateway]
  Gateway --> LB[Load Balancer / RoundRobin]
  LB --> Upstream1[Upstream A]
  LB --> Upstream2[Upstream B]

  Gateway -.->|Health checks| Upstream1
  Gateway -.->|Health checks| Upstream2
  Gateway -->|Prometheus metrics| Prometheus

  subgraph Gateway
    direction TB
    RL[Rate Limiter]
    Auth[JWT Auth]
    Obs[Observability]
    Proxy[Reverse Proxy]
    RL --> Proxy
    Auth --> Proxy
    Obs --> Proxy
  end
```

Components

- Rate limiting: token-bucket per-client/IP
- Health checks: periodic checks and health-aware balancing
- Observability: Prometheus metrics, tracing (JSON)
- Reverse proxy: forwards requests to upstream services
