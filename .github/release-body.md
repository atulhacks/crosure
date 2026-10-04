## Install Crosure {{VERSION}}

**Desktop app**

| Platform | File |
| --- | --- |
| Windows 10/11 (x64) | `Crosure_{{VERSION}}_x64-setup.exe` (or the `.msi`) |
| macOS, Apple Silicon | `Crosure_{{VERSION}}_aarch64.dmg` |
| macOS, Intel | `Crosure_{{VERSION}}_x64.dmg` |
| Debian / Ubuntu | `Crosure_{{VERSION}}_amd64.deb` (`arm64` for ARM) |
| Fedora / RHEL / openSUSE | `Crosure-{{VERSION}}-1.x86_64.rpm` (`aarch64` for ARM) |
| Any Linux | `Crosure_{{VERSION}}_amd64.AppImage` (`aarch64` for ARM): `chmod +x`, then run |

**Command-line tools** (`crosure-cli-{{VERSION}}-<platform>`; no app needed):
- `crosure-reverse` lets an AI reverse a binary from the terminal.
- `crosure-mcp` serves Crosure's recorded tools to any MCP client, such as
  Claude Code.
- `crosure-export` exports recorded investigations as datasets.

**Unsigned builds.** These builds are not yet code-signed.
- **macOS:** right-click the app, choose Open, then Open again. Or run
  `xattr -dr com.apple.quarantine /Applications/Crosure.app`.
- **Windows:** in SmartScreen, choose More info, then Run anyway.

**Verify.** Check a file against `SHA256SUMS.txt`:
`sha256sum -c SHA256SUMS.txt --ignore-missing`.

Optional: install [rizin](https://rizin.re) with `rz-ghidra` for the
decompiler backend.
