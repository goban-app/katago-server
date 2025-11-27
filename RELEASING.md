# Release Process

This document describes how to create a new release of KataGo Server.

## Overview

When you create a git tag, the following happens automatically:

1. **Docker Images**: The CI workflow builds and pushes multi-platform Docker images with version tags
2. **Helm Chart**: The Helm chart is released to GitHub Pages (if chart version changed)
3. **GitHub Release**: A GitHub release is created with release notes

## Release Checklist

### 1. Update Versions

Before creating a release, ensure all version numbers are synchronized:

```bash
# Update these files:
# - Cargo.toml: version = "X.Y.Z"
# - charts/katago-server/Chart.yaml: version = "X.Y.Z" (chart version)
# - charts/katago-server/Chart.yaml: appVersion = "X.Y.Z" (app version)
```

**Important**: The `appVersion` in Chart.yaml should match the version in Cargo.toml.

### 2. Update Cargo.lock

```bash
cargo update -p katago-server
```

### 3. Commit Version Changes

```bash
git add Cargo.toml Cargo.lock charts/katago-server/Chart.yaml
git commit -m "Bump version to X.Y.Z"
git push origin main
```

### 4. Create and Push Tag

```bash
# Create an annotated tag (recommended)
git tag -a vX.Y.Z -m "Release vX.Y.Z"

# Or create a lightweight tag
git tag vX.Y.Z

# Push the tag to trigger release workflows
git push origin vX.Y.Z
```

### 5. Wait for Workflows

The following workflows will run automatically:

- **Release** (`.github/workflows/release.yml`): Builds and pushes Docker images
  - Images are tagged with:
    - `X.Y.Z`, `X.Y`, `X` (semver patterns)
    - `latest`
    - Variant suffixes: `-minimal`, `-base`
  - Platforms: `linux/amd64`, `linux/arm64`
  - Creates GitHub release

- **Helm Release** (`.github/workflows/helm-release.yml`): Releases Helm chart
  - Only runs if `charts/**` directory changed
  - Publishes to GitHub Pages at https://stubbi.github.io/katago-server

### 6. Verify Release

After workflows complete, verify:

1. **Docker Images** are available:
   ```bash
   docker pull ghcr.io/stubbi/katago-server:X.Y.Z
   docker pull ghcr.io/stubbi/katago-server:X.Y.Z-minimal
   docker pull ghcr.io/stubbi/katago-server:X.Y.Z-base
   ```

2. **GitHub Release** is created at: https://github.com/stubbi/katago-server/releases

3. **Helm Chart** is available:
   ```bash
   helm repo add katago-server https://stubbi.github.io/katago-server
   helm repo update
   helm search repo katago-server --versions
   ```

## Version Numbers

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (X): Incompatible API changes
- **MINOR** (Y): New functionality (backwards compatible)
- **PATCH** (Z): Bug fixes (backwards compatible)

## Docker Image Tags

Each release creates the following tags for each variant:

| Variant | Tags |
|---------|------|
| CPU (default) | `X.Y.Z`, `X.Y`, `X`, `latest` |
| Minimal | `X.Y.Z-minimal`, `X.Y-minimal`, `X-minimal`, `latest-minimal` |
| Base | `X.Y.Z-base`, `X.Y-base`, `X-base`, `latest-base` |

## Example Release

To release version 0.2.1:

```bash
# 1. Update versions
vim Cargo.toml              # Set version = "0.2.1"
vim charts/katago-server/Chart.yaml  # Set version = "0.2.2", appVersion = "0.2.1"

# 2. Update lockfile
cargo update -p katago-server

# 3. Commit
git add Cargo.toml Cargo.lock charts/katago-server/Chart.yaml
git commit -m "Bump version to 0.2.1"
git push origin main

# 4. Tag and release
git tag -a v0.2.1 -m "Release v0.2.1"
git push origin v0.2.1

# 5. Wait for workflows and verify
docker pull ghcr.io/stubbi/katago-server:0.2.1
```

## Troubleshooting

### Workflow Failed

- Check the [Actions tab](https://github.com/stubbi/katago-server/actions)
- Common issues:
  - Docker build failures: Check Dockerfile and dependencies
  - Push failures: Verify GITHUB_TOKEN permissions
  - Tag format: Must start with `v` (e.g., `v0.2.1`)

### Image Not Found

- Ensure the release workflow completed successfully
- Check [GitHub Packages](https://github.com/stubbi/katago-server/pkgs/container/katago-server)
- Verify tag matches semver pattern (vX.Y.Z)

### Helm Chart Not Released

- Helm charts only release when `charts/**` files change
- Check if chart version was incremented
- Verify helm-release workflow ran successfully
