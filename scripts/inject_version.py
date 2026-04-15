#!/usr/bin/env python3
"""Inject the CLI version from Cargo.toml into landing page HTML files.

Replaces all occurrences of {{CLI_VERSION}} with the version string
from the workspace Cargo.toml.
"""

import re
from pathlib import Path


def get_cargo_version(cargo_toml: Path) -> str:
    """Extract the version string from Cargo.toml."""
    text = cargo_toml.read_text()
    m = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if not m:
        raise ValueError(f"Could not find version in {cargo_toml}")
    return m.group(1)


def main():
    repo_root = Path(__file__).resolve().parent.parent
    version = get_cargo_version(repo_root / "Cargo.toml")

    landing_dir = repo_root / "landing"
    count = 0
    for html_file in landing_dir.glob("*.html"):
        content = html_file.read_text()
        if "{{CLI_VERSION}}" in content:
            html_file.write_text(content.replace("{{CLI_VERSION}}", version))
            count += 1
            print(f"  Injected v{version} into {html_file.name}")

    if count == 0:
        print("  No {{CLI_VERSION}} placeholders found")
    else:
        print(f"Injected version {version} into {count} file(s)")


if __name__ == "__main__":
    main()
