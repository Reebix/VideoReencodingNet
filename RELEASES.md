# Release Process

This repository now supports automatic Linux binary releases via GitHub Actions.

## How to Create a Release

1. **Create and push a version tag** (starting with `v`):
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. **The GitHub Action will automatically**:
   - Build an optimized release binary for Linux x86-64
   - Create a GitHub release with auto-generated release notes
   - Attach the `VideoReencodingNet` binary as a downloadable asset

## Release Artifacts

Each release will include:
- **VideoReencodingNet**: Linux x86-64 executable binary (~6MB)

## Binary Dependencies

The Linux binary requires:
- Linux x86-64 system
- Standard Linux runtime libraries (glibc)
- FFmpeg/FFprobe installed for video processing functionality

## Usage

Download the binary from the release page and make it executable:
```bash
chmod +x VideoReencodingNet
./VideoReencodingNet --help
```