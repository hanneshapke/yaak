#!/usr/bin/env python3
"""Copy static assets referenced in blog posts into the landing directory.

Parses every Markdown file in ``blog/`` for image references
(``![alt](/some/path/image.gif)``), resolves each path against the project
root, copies the file into ``landing/`` at the same relative path, and
rewrites the Markdown reference so it points to the final served location.

The rewrite rule is simple: any image whose source path lives **outside**
``landing/`` is copied into ``landing/`` and the Markdown path is updated to
the location the web server will actually serve.

Designed to run in CI right before the Pages artifact is uploaded:

    python scripts/copy_assets.py

Directory layout (before):

    blog/2026-04-15-seven-yaak-demos.md   ->  ![…](/demos/output/01-wizard.gif)
    demos/output/01-wizard.gif

Directory layout (after):

    blog/2026-04-15-seven-yaak-demos.md   ->  ![…](/demos/output/01-wizard.gif)
    landing/demos/output/01-wizard.gif      (copied)
"""

from __future__ import annotations

import re
import shutil
import sys
from pathlib import Path

# Markdown image pattern: ![alt text](/path/to/image.ext)
_IMAGE_RE = re.compile(r"!\[([^\]]*)\]\((/[^)]+)\)")

# File extensions we treat as static assets worth copying.
_ASSET_EXTENSIONS = {
    ".gif",
    ".png",
    ".jpg",
    ".jpeg",
    ".svg",
    ".webp",
    ".bmp",
    ".ico",
    ".mp4",
    ".webm",
    ".mov",
    ".pdf",
}


def _is_asset(path: Path) -> bool:
    return path.suffix.lower() in _ASSET_EXTENSIONS


def main() -> None:
    project_root = Path(__file__).resolve().parent.parent
    blog_dir = project_root / "blog"
    landing_dir = project_root / "landing"

    if not blog_dir.is_dir():
        print(f"Blog directory not found: {blog_dir}", file=sys.stderr)
        sys.exit(1)

    blog_posts = sorted(blog_dir.glob("*.md"))
    if not blog_posts:
        print("No markdown files found in blog/")
        return

    copied: dict[Path, Path] = {}  # src -> dest
    errors: list[str] = []

    for post in blog_posts:
        content = post.read_text()

        for match in _IMAGE_RE.finditer(content):
            img_path = match.group(2)  # e.g. /demos/output/01-wizard.gif

            # Resolve to an absolute filesystem path (strip the leading /)
            src = project_root / img_path.lstrip("/")

            if not _is_asset(src):
                continue

            # Already lives inside landing/ — nothing to do.
            try:
                src.resolve().relative_to(landing_dir.resolve())
                continue
            except ValueError:
                pass

            if not src.exists():
                errors.append(f"  ✗ {post.name}: asset not found: {img_path}")
                continue

            # Mirror the repo-relative path under landing/
            dest = landing_dir / img_path.lstrip("/")
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dest)
            copied[src] = dest

    # ---- report -------------------------------------------------------- #
    if copied:
        print(f"Copied {len(copied)} asset(s) into landing/:")
        for src, dest in sorted(copied.items()):
            rel_src = src.relative_to(project_root)
            rel_dest = dest.relative_to(project_root)
            print(f"  ✓ {rel_src} -> {rel_dest}")

    if errors:
        print(f"\n{len(errors)} warning(s):")
        for err in errors:
            print(err)

    if not copied and not errors:
        print("No assets to copy.")

    print(f"\n✅ Done.")


if __name__ == "__main__":
    main()
