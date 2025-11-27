# KataGo Server Helm Chart

A Helm chart for deploying [KataGo Server](https://github.com/stubbi/katago-server) on Kubernetes.

## Introduction

This chart bootstraps a KataGo Server deployment on a Kubernetes cluster using the Helm package manager.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.0+
- (Optional) GPU support with NVIDIA device plugin for GPU variants

## Installation

### Add Helm Repository

```bash
helm repo add katago-server https://stubbi.github.io/katago-server
helm repo update
```

### Install Chart

```bash
# Install with default values (CPU variant)
helm install my-katago-server katago-server/katago-server

# Install with custom values
helm install my-katago-server katago-server/katago-server -f my-values.yaml

# Install in a specific namespace
helm install my-katago-server katago-server/katago-server --namespace katago --create-namespace
```

## Uninstallation

```bash
helm uninstall my-katago-server
```

## Configuration

The following table lists the configurable parameters of the KataGo Server chart and their default values.

### Common Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of replicas | `1` |
| `image.repository` | Image repository | `ghcr.io/stubbi/katago-server` |
| `image.tag` | Image tag | `latest` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `image.variant` | Image variant (empty for CPU, `-minimal` for minimal) | `""` |

### Service Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `service.type` | Kubernetes service type | `ClusterIP` |
| `service.port` | Service port | `2718` |
| `service.targetPort` | Container port | `2718` |

### Ingress Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `ingress.enabled` | Enable ingress | `false` |
| `ingress.className` | Ingress class name | `""` |
| `ingress.annotations` | Ingress annotations | `{}` |
| `ingress.hosts` | Ingress hosts configuration | See values.yaml |
| `ingress.tls` | Ingress TLS configuration | `[]` |

### Resource Management

| Parameter | Description | Default |
|-----------|-------------|---------|
| `resources` | CPU/Memory resource requests/limits | `{}` |
| `autoscaling.enabled` | Enable Horizontal Pod Autoscaler | `false` |
| `autoscaling.minReplicas` | Minimum number of replicas | `1` |
| `autoscaling.maxReplicas` | Maximum number of replicas | `10` |
| `autoscaling.targetCPUUtilizationPercentage` | Target CPU utilization | `80` |

### GPU Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `gpu.enabled` | Enable GPU support | `false` |
| `gpu.count` | Number of GPUs to request | `1` |
| `gpu.vendor` | GPU vendor (nvidia/amd) | `nvidia` |

### Application Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `config.logLevel` | Log level (trace/debug/info/warn/error) | `info` |
| `config.customConfig` | Custom config.toml content | `""` |
| `config.katago.path` | KataGo binary path (minimal variant) | `/models/katago` |
| `config.katago.modelPath` | Model file path (minimal variant) | `/models/model.bin.gz` |
| `config.katago.configPath` | Analysis Engine config path (minimal variant) | `/models/analysis_config.cfg` |
| `config.katago.moveTimeoutSecs` | Move timeout in seconds | `20` |

### Custom Model Download

| Parameter | Description | Default |
|-----------|-------------|---------|
| `config.customModel.enabled` | Enable custom model download via init container | `false` |
| `config.customModel.url` | URL to download the model from | `""` |
| `config.customModel.filename` | Filename to save the model as | `"custom-model.bin.gz"` |
| `config.customModel.sha256sum` | Optional SHA256 checksum for validation | `""` |
| `config.customModel.initImage` | Init container image for downloading | `"busybox:1.36"` |
| `config.customModel.initResources` | Resource limits for init container | See values.yaml |

## Examples

### Basic CPU Deployment

```yaml
# values-cpu.yaml
replicaCount: 1

image:
  tag: "latest"
  variant: ""

resources:
  limits:
    cpu: 4000m
    memory: 2Gi
  requests:
    cpu: 2000m
    memory: 1Gi

config:
  logLevel: info
```

Install:
```bash
helm install katago katago-server/katago-server -f values-cpu.yaml
```

### CPU Deployment with Custom Model Download

Download a custom KataGo model automatically via init container:

```yaml
# values-custom-model.yaml
replicaCount: 1

image:
  tag: "latest"
  variant: ""

resources:
  limits:
    cpu: 4000m
    memory: 2Gi
  requests:
    cpu: 2000m
    memory: 1Gi

config:
  logLevel: info
  customModel:
    enabled: true
    # Download a stronger 40-block model
    url: "https://katagotraining.org/api/networks/kata1-b40c256-s11840935168-d2898845681/network_file"
    filename: "kata1-b40c256.bin.gz"
    # Optional: Add checksum for validation
    # sha256sum: "your-sha256-checksum-here"
  customConfig: |
    [server]
    host = "0.0.0.0"
    port = 2718

    [katago]
    katago_path = "./katago"
    model_path = "/app/models/kata1-b40c256.bin.gz"
    config_path = "./gtp_config.cfg"
    move_timeout_secs = 30
```

Install:
```bash
helm install katago katago-server/katago-server -f values-custom-model.yaml
```

The init container will:
1. Download the model from the specified URL before the main container starts
2. Optionally verify the SHA256 checksum
3. Place the model at `/app/models/<filename>` (mounted as emptyDir)
4. The main container will have read-only access to the downloaded model

### GPU Deployment

```yaml
# values-gpu.yaml
image:
  tag: "gpu"  # Use your locally built GPU image
  variant: ""

gpu:
  enabled: true
  count: 1
  vendor: nvidia

resources:
  limits:
    memory: 4Gi
  requests:
    cpu: 2000m
    memory: 2Gi

nodeSelector:
  nvidia.com/gpu: "true"

tolerations:
  - key: nvidia.com/gpu
    operator: Exists
    effect: NoSchedule
```

Install:
```bash
helm install katago katago-server/katago-server -f values-gpu.yaml
```

### Production Deployment with Ingress

```yaml
# values-production.yaml
replicaCount: 3

image:
  tag: "v0.1.0"

resources:
  limits:
    cpu: 4000m
    memory: 2Gi
  requests:
    cpu: 2000m
    memory: 1Gi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70

podDisruptionBudget:
  enabled: true
  minAvailable: 2

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
  hosts:
    - host: katago.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: katago-tls
      hosts:
        - katago.example.com

config:
  logLevel: warn

serviceMonitor:
  enabled: true
  interval: 30s
```

Install:
```bash
helm install katago katago-server/katago-server -f values-production.yaml
```

### Minimal Variant with Custom Model

```yaml
# values-minimal.yaml
image:
  tag: "latest"
  variant: "-minimal"

volumes:
  - name: katago-models
    persistentVolumeClaim:
      claimName: katago-models-pvc

volumeMounts:
  - name: katago-models
    mountPath: /models
    readOnly: true

config:
  customConfig: |
    [server]
    host = "0.0.0.0"
    port = 2718

    [katago]
    katago_path = "/models/katago"
    model_path = "/models/kata1-b40c256-s11840935168-d2898845681.bin.gz"
    config_path = "/models/gtp_config.cfg"
    move_timeout_secs = 30
```

Install:
```bash
# First, create PVC with your models
kubectl create -f katago-models-pvc.yaml

# Then install the chart
helm install katago katago-server/katago-server -f values-minimal.yaml
```

## Health Checks

The chart includes comprehensive health checks:

- **Liveness Probe**: Checks if the application is running (30s initial delay)
- **Readiness Probe**: Checks if the application is ready to serve traffic (15s initial delay)
- **Startup Probe**: Allows extra time for the application to start (10s initial delay, up to 60s total)

All probes use the `/health` endpoint.

## Monitoring

To enable Prometheus monitoring:

```yaml
serviceMonitor:
  enabled: true
  interval: 30s
  labels:
    prometheus: kube-prometheus
```

Note: This requires the Prometheus Operator to be installed in your cluster.

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -l app.kubernetes.io/name=katago-server
kubectl describe pod <pod-name>
kubectl logs <pod-name>
```

### Test the Service

```bash
# Port forward to test locally
kubectl port-forward svc/my-katago-server 2718:2718

# Health check
curl http://localhost:2718/health

# API test
curl -X POST http://localhost:2718/select-move/katago_gtp_bot \
  -H "Content-Type: application/json" \
  -d '{"board_size":19,"moves":["R4","D16"],"config":{"komi":7.5}}'
```

### Common Issues

1. **Pod not starting**: Check resource limits and ensure the node has enough resources
2. **GPU not detected**: Ensure NVIDIA device plugin is installed and GPU nodes are labeled
3. **Timeout errors**: Increase `config.katago.moveTimeoutSecs` or reduce model complexity
4. **OOM errors**: Increase memory limits in `resources.limits.memory`

## Upgrading

```bash
# Update the Helm repository
helm repo update

# Check for available versions
helm search repo katago-server --versions

# Upgrade to a new version
helm upgrade my-katago-server katago-server/katago-server --version X.Y.Z

# Upgrade with new values
helm upgrade my-katago-server katago-server/katago-server -f new-values.yaml
```

## Development

### Testing Locally

```bash
# Lint the chart
helm lint charts/katago-server

# Template the chart (dry-run)
helm template my-katago-server charts/katago-server -f values.yaml

# Install in test mode
helm install my-katago-server charts/katago-server --dry-run --debug
```

### Package the Chart

```bash
helm package charts/katago-server
```

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/stubbi/katago-server) for contribution guidelines.

## License

This chart is provided under the same license as the KataGo Server project. See the [LICENSE](https://github.com/stubbi/katago-server/blob/main/LICENSE) file for details.

## References

- [KataGo Server Repository](https://github.com/stubbi/katago-server)
- [KataGo](https://github.com/lightvector/KataGo)
- [Helm Documentation](https://helm.sh/docs/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
