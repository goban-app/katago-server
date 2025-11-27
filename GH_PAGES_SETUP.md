# GitHub Pages Setup for Helm Repository

## Current Status

The `gh-pages` branch has been cleaned up locally and is ready to be pushed. The cleanup includes:

### ✅ Changes Made Locally

1. **Removed all source code files** from gh-pages branch
2. **Created `index.html`** - A landing page with instructions for adding the Helm repository
3. **Created `index.yaml`** - Empty Helm repository index (will be populated by helm-release.yml workflow)

### 📋 Files Now on gh-pages Branch

```
gh-pages/
├── index.html    # Landing page for users visiting the repository URL
└── index.yaml    # Helm repository index (empty, will be populated by releases)
```

## Required Action

The gh-pages branch changes are committed locally but need to be pushed manually:

```bash
# Switch to gh-pages branch
git checkout gh-pages

# Verify the changes
git log -1
git status

# Push to remote
git push origin gh-pages
```

Expected commit message:
```
Initialize gh-pages as proper Helm chart repository

- Remove all source code files
- Add index.html landing page with usage instructions
- Initialize empty index.yaml for Helm repository
- gh-pages will now only contain Helm chart packages and repository metadata
```

## How It Works

### 1. GitHub Pages Configuration

Ensure GitHub Pages is configured to serve from the `gh-pages` branch:
- Repository Settings → Pages
- Source: Deploy from a branch
- Branch: `gh-pages` / `/ (root)`

### 2. Helm Release Workflow

The `.github/workflows/helm-release.yml` workflow will:

1. **Trigger on:**
   - Git tags matching `v*.*.*` (e.g., `v0.1.0`)
   - Manual workflow dispatch

2. **Process:**
   - Package the Helm chart
   - Checkout gh-pages branch
   - Copy packaged `.tgz` files to gh-pages
   - Update `index.yaml` with chart metadata
   - Commit and push to gh-pages
   - Create GitHub release with chart package

3. **Result:**
   - Chart available at: `https://stubbi.github.io/katago-server/`
   - Users can add with: `helm repo add katago-server https://stubbi.github.io/katago-server`

### 3. Workflow Verification

The workflow is already correctly configured and will NOT accidentally add source code back to gh-pages because:
- It uses `actions/checkout@v4` with `path: gh-pages` (checks out into subdirectory)
- Only copies `.tgz` files from `.cr-release-packages/` to `gh-pages/`
- Only modifies `index.yaml` via `helm repo index`
- Works within the `gh-pages` working directory

## Testing the Setup

Once gh-pages is pushed:

1. **Visit the landing page:**
   ```
   https://stubbi.github.io/katago-server/
   ```

2. **Create a test release:**
   ```bash
   # Tag and push to trigger helm-release workflow
   git tag v0.1.0
   git push origin v0.1.0
   ```

3. **Verify the chart is published:**
   ```bash
   helm repo add katago-server https://stubbi.github.io/katago-server
   helm repo update
   helm search repo katago-server
   ```

## Future Releases

To release a new chart version:

```bash
# Option 1: Create a git tag (auto-releases)
git tag v0.2.0
git push origin v0.2.0

# Option 2: Manual workflow dispatch
# Go to Actions → Helm Chart Release → Run workflow
# Enter chart version (e.g., 0.2.0)
```

The workflow will automatically:
- Package the chart with the specified version
- Update Chart.yaml with the version
- Publish to gh-pages
- Create a GitHub release
