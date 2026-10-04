# Releases

Each release is built by GitHub Actions (`.github/workflows/release.yml`)
for five platforms:

| Platform | Desktop app | Command-line tools |
| --- | --- | --- |
| Linux x64 | `.deb`, `.rpm`, `.AppImage` | `crosure-cli-<v>-linux-x64.tar.gz` |
| Linux arm64 | `.deb`, `.rpm`, `.AppImage` | `crosure-cli-<v>-linux-arm64.tar.gz` |
| macOS Apple Silicon | `.dmg` | `crosure-cli-<v>-macos-arm64.tar.gz` |
| macOS Intel | `.dmg` | `crosure-cli-<v>-macos-x64.tar.gz` |
| Windows x64 | `.msi`, setup `.exe` | `crosure-cli-<v>-windows-x64.zip` |

The command-line archive holds:
- `crosure-reverse`, the AI agent in a terminal;
- `crosure-mcp`, the MCP server;
- `crosure-export`, the dataset exporter.

They need no system libraries. `SHA256SUMS.txt` covers every file.

Linux builds run on Ubuntu 22.04, so the packages work on that release and
newer (glibc 2.35). macOS builds need macOS 11 or later.

## Cutting a release

```bash
python3 tools/version.py set 0.2.0      # Cargo, Cargo.lock, Tauri, npm
git commit -am "Release 0.2.0"
git tag v0.2.0
git push origin HEAD v0.2.0
```

The tag starts the workflow:
1. **prepare** checks that the tag matches the version in the files. A
   mismatch stops the release before anything is built. It then creates a
   **draft** release, with install notes (`.github/release-body.md`) followed
   by the generated changelog.
2. **app** builds the installers on each platform and attaches them.
3. **cli** builds the command-line tools and attaches the archives.
4. **checksums** attaches `SHA256SUMS.txt`.

Review the draft on GitHub, then press **Publish**. A version with a suffix
(`0.2.0-rc.1`) is marked as a pre-release. Re-running a failed workflow
reuses the same draft, and files are replaced, not duplicated.

**Dry run.** Use Actions → Release → Run workflow. It builds the same files
without a release and keeps them as workflow artifacts for 90 days.

CI also checks that every version file agrees (`python3 tools/version.py`).

## Code signing

The builds are not signed yet. The macOS app is ad-hoc signed
(`signingIdentity: "-"`), which Apple Silicon requires before an app will
run. Gatekeeper still asks for confirmation on first launch, and Windows
SmartScreen warns. The release notes explain how to proceed.

To sign, add these repository secrets and pass them to the `tauri-action`
step as `env`:
- **macOS:** `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`, and for notarization `APPLE_ID`,
  `APPLE_PASSWORD` and `APPLE_TEAM_ID`.
- **Windows:** a certificate thumbprint in `bundle.windows`, or Azure
  Trusted Signing.

Do not set those variables to empty values. Tauri treats a set variable as a
request to sign, and an empty one fails the build.

## Building locally

```bash
cd tauri && npm ci && npx tauri build                 # this machine's installers
cargo build --release --bins -p crosure-agent -p crosure-mcp -p crosure-dataset
```

Installers are written to `target/release/bundle/`, or
`target/<triple>/release/bundle/` when `--target` is given.
