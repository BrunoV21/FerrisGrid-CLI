# FerrisGrid Releases and Deployments

This document tracks the release and deployment setup for FerrisGrid.

Last updated: 2026-06-02

## Current automation

| Workflow | File | Trigger | Output |
| --- | --- | --- | --- |
| CI | `.github/workflows/ci.yml` | Pull requests, pushes to `main`, manual runs | Rust checks, docs build, Dockerfile build check |
| Release | `.github/workflows/release.yml` | Tags matching `v*.*.*`, manual runs | GitHub Release assets and `SHA256SUMS` |
| Docs Deploy | `.github/workflows/docs.yml` | Docs changes on `main`, manual runs | GitHub Pages deployment |
| Docker Publish | `.github/workflows/docker.yml` | Pushes to `main`, release tags, manual runs | GHCR Linux workspace image |

## Required GitHub settings

These settings are required before the workflows can publish successfully.

1. Enable GitHub Actions for the repository.
2. In `Settings -> Actions -> General`, allow workflows to create and approve releases/packages according to the repository policy.
3. In `Settings -> Actions -> General -> Workflow permissions`, use read/write permissions, or ensure the workflow-level `contents: write`, `packages: write`, `pages: write`, and `id-token: write` permissions are allowed.
4. In `Settings -> Pages`, set the source to `GitHub Actions`.
5. After the first GHCR publish, set the container package visibility to public if public Docker pulls should work without authentication.

## Required secrets and variables

The current workflows do not require manually created repository secrets.

| Name | Type | Required now | Used by | Notes |
| --- | --- | --- | --- | --- |
| `GITHUB_TOKEN` | GitHub-provided token | Yes, automatic | Release, Docs Deploy, Docker Publish | GitHub injects this automatically; do not create it manually. |
| `REGISTRY` | Workflow env var | Yes, defined in workflow | Docker Publish | Currently `ghcr.io`. |
| `IMAGE_NAME` | Workflow env var | Yes, defined in workflow | Docker Publish | Currently `${{ github.repository }}/linux-workspace`. |
| `CARGO_TERM_COLOR` | Workflow env var | Yes, defined in workflow | Release | Keeps Cargo output readable in logs. |

## Future distribution secrets

Add these only when the matching distribution channel is implemented.

| Name | Type | Needed for | Notes |
| --- | --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | Repository secret | Publishing Rust crates to crates.io | Create from the crates.io account that owns the crates. |
| `HOMEBREW_TAP_TOKEN` | Repository secret | Updating a Homebrew tap repo | Needs write access to the tap repository, for example `BrunoV21/homebrew-ferrisgrid`. |
| `DOCKERHUB_USERNAME` | Repository variable | Optional Docker Hub publishing | Only needed if publishing outside GHCR. |
| `DOCKERHUB_TOKEN` | Repository secret | Optional Docker Hub publishing | Use a Docker Hub access token, not an account password. |
| `COSIGN_PRIVATE_KEY` | Repository secret | Optional artifact/container signing | Only needed if release signing is added. |
| `COSIGN_PASSWORD` | Repository secret | Optional artifact/container signing | Password for the signing key, if using key-based signing. |

## Release process

Use semantic version tags prefixed with `v`.

```bash
git tag v0.1.0
git push origin v0.1.0
```

The tag triggers:

1. Native release builds for Linux x64, Linux arm64, macOS Intel, macOS Apple Silicon, and Windows x64.
2. Fake-backend smoke tests for each binary.
3. Archive packaging.
4. `SHA256SUMS` generation.
5. GitHub Release publishing.
6. GHCR Docker image publishing from the Docker workflow.

The release assets are named:

```text
ferrisgrid-x86_64-unknown-linux-gnu.tar.gz
ferrisgrid-aarch64-unknown-linux-gnu.tar.gz
ferrisgrid-x86_64-apple-darwin.tar.gz
ferrisgrid-aarch64-apple-darwin.tar.gz
ferrisgrid-x86_64-pc-windows-msvc.zip
SHA256SUMS
```

## Manual release rerun

If the tag already exists and a release needs to be repackaged, run the `Release` workflow manually with the tag input:

```text
v0.1.0
```

The workflow uploads assets with `--clobber`, so existing release assets with the same names are replaced.

## Docs deployment

Docs deploy from `docs/official` through VitePress.

The workflow runs:

```bash
npm ci
npm run docs:build
```

The generated site is uploaded from:

```text
docs/official/.vitepress/dist
```

## Docker image

The Docker workflow publishes the Linux workspace image to:

```text
ghcr.io/<owner>/<repo>/linux-workspace
```

Expected public pull command after package visibility is public:

```bash
docker pull ghcr.io/<owner>/<repo>/linux-workspace:latest
```

Release tags also produce versioned image tags such as:

```text
ghcr.io/<owner>/<repo>/linux-workspace:v0.1.0
```

## Local preflight checks

Run these before tagging a release:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo build --release --locked -p ferrisgrid-cli
cd docs/official
npm ci
npm run docs:build
```

If Docker is running locally, also run:

```bash
docker build -f docker/linux-workspace.Dockerfile -t ferrisgrid-linux-workspace:ci .
```
