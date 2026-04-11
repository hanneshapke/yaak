#!/usr/bin/env python3
"""Build landing/changelog.html from CHANGELOG.md."""

import html
import re
from datetime import datetime
from pathlib import Path


def parse_changelog(text: str) -> list[dict]:
    """Parse Keep a Changelog markdown into structured data.

    Returns a list of release dicts:
      {
        "version": str,        # e.g. "0.0.1" or "Unreleased"
        "date": str | None,    # e.g. "2026-04-04" or None
        "groups": [{"label": str, "items": [str]}]
      }
    """
    releases = []
    current_release = None
    current_group = None

    for line in text.splitlines():
        # Release heading: ## [0.0.1] — 2026-04-04  or  ## [Unreleased]
        m = re.match(r"^## \[(.+?)\](?:\s*(?:—|-)\s*(\d{4}-\d{2}-\d{2}))?", line)
        if m:
            if current_release is not None:
                if current_group:
                    current_release["groups"].append(current_group)
                releases.append(current_release)
            current_release = {
                "version": m.group(1),
                "date": m.group(2),
                "groups": [],
            }
            current_group = None
            continue

        # Change group heading: ### Added, ### Changed, ### Fixed, etc.
        m = re.match(r"^### (.+)", line)
        if m and current_release is not None:
            if current_group:
                current_release["groups"].append(current_group)
            current_group = {"label": m.group(1).strip(), "items": []}
            continue

        # List item: - Some description
        m = re.match(r"^- (.+)", line)
        if m and current_group is not None:
            current_group["items"].append(m.group(1))
            continue

    # Flush last release/group
    if current_release is not None:
        if current_group:
            current_release["groups"].append(current_group)
        releases.append(current_release)

    return releases


def format_inline(text: str) -> str:
    """Convert markdown inline formatting to HTML."""
    # Escape HTML entities first
    text = html.escape(text)
    # Convert backtick code spans
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    # Convert markdown links [text](url)
    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', text)
    return text


def format_date_long(date_str: str) -> str:
    """Format '2026-04-04' as 'April 4, 2026'."""
    dt = datetime.strptime(date_str, "%Y-%m-%d")
    return dt.strftime("%B %-d, %Y")


def format_date_short(date_str: str) -> str:
    """Format '2026-04-04' as 'Apr 4'."""
    dt = datetime.strptime(date_str, "%Y-%m-%d")
    return dt.strftime("%b %-d")


def release_id(version: str) -> str:
    if version.lower() == "unreleased":
        return "unreleased"
    return f"v{version}"


def render_toc(releases: list[dict]) -> str:
    items = []
    for r in releases:
        rid = release_id(r["version"])
        if r["version"].lower() == "unreleased":
            version_text = "Unreleased"
            date_text = "\u2014"
        else:
            version_text = r["version"]
            date_text = format_date_short(r["date"]) if r["date"] else "\u2014"
        items.append(
            f'        <li class="toc-item">\n'
            f'          <a href="#{rid}">\n'
            f'            <span class="toc-version">{version_text}</span>\n'
            f'            <span class="toc-date">{date_text}</span>\n'
            f"          </a>\n"
            f"        </li>"
        )
    return "\n".join(items)


def render_change_group(group: dict) -> str:
    label = group["label"]
    label_lower = label.lower()
    items_html = []
    for item in group["items"]:
        items_html.append(
            f"            <li class=\"change-item\">\n"
            f'              <span class="change-bullet">&rsaquo;</span>\n'
            f"              <span>{format_inline(item)}</span>\n"
            f"            </li>"
        )
    return (
        f'        <div class="change-group">\n'
        f'          <div class="change-group-label label-{label_lower}">'
        f'<span class="dot"></span> {label}</div>\n'
        f'          <ul class="change-list">\n'
        + "\n".join(items_html)
        + "\n"
        f"          </ul>\n"
        f"        </div>"
    )


def render_release(release: dict, is_latest: bool) -> str:
    rid = release_id(release["version"])
    is_unreleased = release["version"].lower() == "unreleased"

    # Header
    version_html = f'<span class="release-version">{release["version"] if not is_unreleased else "Unreleased"}</span>'

    date_html = ""
    if release["date"] and not is_unreleased:
        date_html = f'\n          <span class="release-date">{format_date_long(release["date"])}</span>'

    tag_html = ""
    if is_unreleased:
        tag_html = '\n          <span class="release-tag unreleased">In development</span>'
    elif is_latest:
        tag_html = '\n          <span class="release-tag latest">Latest</span>'

    groups_html = "\n".join(render_change_group(g) for g in release["groups"])

    return (
        f'      <article class="release" id="{rid}">\n'
        f"        <div class=\"release-header\">\n"
        f"          {version_html}{date_html}{tag_html}\n"
        f"        </div>\n"
        f"{groups_html}\n"
        f"      </article>"
    )


def render_html(releases: list[dict]) -> str:
    # Determine which release gets the "latest" tag (first non-unreleased)
    latest_idx = None
    for i, r in enumerate(releases):
        if r["version"].lower() != "unreleased":
            latest_idx = i
            break

    toc_html = render_toc(releases)

    releases_html = []
    for i, r in enumerate(releases):
        releases_html.append(render_release(r, is_latest=(i == latest_idx)))
    releases_joined = "\n\n".join(releases_html)

    return f"""\
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>yaak &mdash; changelog</title>
<link href="https://fonts.googleapis.com/css2?family=Playfair+Display:ital,wght@0,400;0,700;0,900;1,400;1,700&family=DM+Mono:wght@300;400;500&family=Source+Serif+4:ital,opsz,wght@0,8..60,300;0,8..60,400;0,8..60,600;1,8..60,300;1,8..60,400&display=swap" rel="stylesheet">
<link rel="stylesheet" href="style.css">
<style>
  :root {{
    --added: #3a7d44;
    --changed: #b47d2e;
    --fixed: #5a6abf;
  }}

  /* ============ HERO ============ */
  .page-hero {{
    padding: 80px 32px 48px;
    border-bottom: 3px double var(--rule);
  }}

  .page-hero-inner {{
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: end;
    gap: 32px;
  }}

  .page-kicker {{
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 3px;
    color: var(--red);
    margin-bottom: 16px;
    animation: fadeIn 0.5s ease both;
  }}

  .page-title {{
    font-family: 'Playfair Display', serif;
    font-weight: 900;
    font-size: clamp(40px, 7vw, 72px);
    line-height: 0.95;
    letter-spacing: -2px;
    animation: fadeIn 0.5s ease 0.05s both;
  }}
  .page-title em {{ font-style: italic; font-weight: 400; color: var(--red); }}

  .page-meta {{
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    color: var(--ink-faint);
    text-align: right;
    letter-spacing: 0.5px;
    line-height: 1.8;
    animation: fadeIn 0.5s ease 0.1s both;
  }}

  /* ============ TOC (sidebar) ============ */
  .changelog-layout {{
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 48px;
    padding: 48px 32px 100px;
  }}

  .toc {{
    position: sticky;
    top: 32px;
    align-self: start;
    animation: fadeIn 0.5s ease 0.15s both;
  }}

  .toc-label {{
    font-family: 'DM Mono', monospace;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 2.5px;
    color: var(--ink-faint);
    margin-bottom: 16px;
  }}

  .toc-list {{
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0;
  }}

  .toc-item a {{
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 0;
    border-bottom: 1px solid var(--rule);
    text-decoration: none;
    font-family: 'DM Mono', monospace;
    font-size: 13px;
    color: var(--ink-dim);
    transition: color 0.2s;
  }}
  .toc-item a:hover {{ color: var(--red); }}
  .toc-item:first-child a {{ border-top: 1px solid var(--rule); }}

  .toc-version {{ font-weight: 500; color: var(--ink); }}
  .toc-date {{ font-size: 10px; color: var(--ink-faint); margin-left: auto; }}

  /* ============ RELEASE ENTRIES ============ */
  .releases {{
    display: flex;
    flex-direction: column;
    gap: 0;
  }}

  .release {{
    border-bottom: 1px solid var(--rule);
    padding: 40px 0;
    animation: fadeIn 0.5s ease both;
  }}
  .release:first-child {{ padding-top: 0; }}

  .release-header {{
    display: flex;
    align-items: baseline;
    gap: 16px;
    margin-bottom: 28px;
    flex-wrap: wrap;
  }}

  .release-version {{
    font-family: 'Playfair Display', serif;
    font-size: 32px;
    font-weight: 900;
    letter-spacing: -0.5px;
  }}

  .release-date {{
    font-family: 'DM Mono', monospace;
    font-size: 12px;
    color: var(--ink-faint);
    letter-spacing: 0.5px;
  }}

  .release-tag {{
    font-family: 'DM Mono', monospace;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    padding: 4px 12px;
    border-radius: 100px;
    border: 1px solid;
  }}
  .release-tag.latest {{
    color: var(--red);
    border-color: var(--red);
    background: rgba(196, 66, 58, 0.06);
  }}
  .release-tag.unreleased {{
    color: var(--ink-faint);
    border-color: var(--rule);
  }}

  .change-group {{
    margin-bottom: 24px;
  }}
  .change-group:last-child {{ margin-bottom: 0; }}

  .change-group-label {{
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 2px;
    margin-bottom: 12px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }}

  .change-group-label .dot {{
    width: 7px;
    height: 7px;
    border-radius: 50%;
    display: inline-block;
  }}

  .label-added {{ color: var(--added); }}
  .label-added .dot {{ background: var(--added); }}
  .label-changed {{ color: var(--changed); }}
  .label-changed .dot {{ background: var(--changed); }}
  .label-fixed {{ color: var(--fixed); }}
  .label-fixed .dot {{ background: var(--fixed); }}

  .change-list {{
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0;
  }}

  .change-item {{
    display: grid;
    grid-template-columns: 20px 1fr;
    gap: 8px;
    padding: 8px 0;
    border-bottom: 1px solid rgba(209, 201, 192, 0.4);
    font-size: 14px;
    line-height: 1.65;
    color: var(--ink-light);
    font-weight: 300;
  }}
  .change-item:last-child {{ border-bottom: none; }}

  .change-bullet {{
    font-family: 'Playfair Display', serif;
    color: var(--ink-faint);
    font-size: 16px;
    line-height: 1.55;
    user-select: none;
  }}

  .change-item code {{
    font-family: 'DM Mono', monospace;
    font-size: 12px;
    background: var(--code-bg);
    color: var(--code-green);
    padding: 1px 6px;
    border-radius: 3px;
  }}

  /* Stagger the releases */
  .release:nth-child(1) {{ animation-delay: 0.15s; }}
  .release:nth-child(2) {{ animation-delay: 0.25s; }}
  .release:nth-child(3) {{ animation-delay: 0.35s; }}
  .release:nth-child(4) {{ animation-delay: 0.45s; }}

  /* ============ RESPONSIVE ============ */
  @media (max-width: 768px) {{
    .changelog-layout {{
      grid-template-columns: 1fr;
      gap: 32px;
    }}
    .toc {{ position: static; }}
    .page-hero-inner {{ grid-template-columns: 1fr; }}
    .page-meta {{ text-align: left; }}
    .release-header {{ flex-direction: column; gap: 8px; }}
  }}
</style>
</head>
<body>

<!-- NAV -->
<nav>
  <a href="index.html" class="nav-brand">yaak</a>
  <button class="hamburger" id="hamburger" aria-label="Open menu" aria-expanded="false">
    <span></span><span></span><span></span>
  </button>
  <div class="nav-right">
    <a href="index.html">Home</a>
    <a href="docs.html">Docs</a>
    <a href="blog.html">Blog</a>
    <a href="changelog.html" class="active">Changelog</a>
    <a href="https://github.com/hanneshapke/yaak" class="nav-gh">
      <svg width="14" height="14" fill="currentColor" viewBox="0 0 16 16"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
      Star on GitHub
    </a>
  </div>
</nav>

<!-- MOBILE MENU OVERLAY -->
<div class="mobile-menu" id="mobileMenu">
  <a href="index.html" class="mobile-menu-link">Home</a>
  <a href="docs.html" class="mobile-menu-link">Docs</a>
  <a href="blog.html" class="mobile-menu-link">Blog</a>
  <a href="changelog.html" class="mobile-menu-link">Changelog</a>
  <a href="https://github.com/hanneshapke/yaak" class="mobile-menu-link nav-gh">
    <svg width="14" height="14" fill="currentColor" viewBox="0 0 16 16"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
    Star on GitHub
  </a>
</div>

<!-- PAGE HERO -->
<section class="page-hero">
  <div class="container">
    <div class="page-hero-inner">
      <div>
        <div class="page-kicker">Release history</div>
        <h1 class="page-title">Change<em>log</em></h1>
      </div>
      <div class="page-meta">
        Format: Keep a Changelog<br>
        Versioning: SemVer
      </div>
    </div>
  </div>
</section>

<!-- CONTENT -->
<div class="container">
  <div class="changelog-layout">

    <!-- TOC -->
    <aside class="toc">
      <div class="toc-label">Releases</div>
      <ul class="toc-list">
{toc_html}
      </ul>
    </aside>

    <!-- RELEASES -->
    <main class="releases">

{releases_joined}

    </main>
  </div>
</div>

<!-- FOOTER -->
<footer>
  <span>yaak &mdash; open source, MIT licensed</span>
  <span>Built with Rust &amp; <a href="#">your favorite LLM</a></span>
</footer>

<script>
// Hamburger menu toggle
const hamburger = document.getElementById('hamburger');
const mobileMenu = document.getElementById('mobileMenu');

hamburger.addEventListener('click', () => {{
  const isOpen = mobileMenu.classList.toggle('open');
  hamburger.classList.toggle('active');
  hamburger.setAttribute('aria-expanded', isOpen);
  document.body.style.overflow = isOpen ? 'hidden' : '';
}});

mobileMenu.querySelectorAll('.mobile-menu-link').forEach(link => {{
  link.addEventListener('click', () => {{
    mobileMenu.classList.remove('open');
    hamburger.classList.remove('active');
    hamburger.setAttribute('aria-expanded', 'false');
    document.body.style.overflow = '';
  }});
}});
</script>

</body>
</html>
"""


def main():
    repo_root = Path(__file__).resolve().parent.parent
    changelog_md = repo_root / "CHANGELOG.md"
    output_html = repo_root / "landing" / "changelog.html"

    releases = parse_changelog(changelog_md.read_text())
    output = render_html(releases)
    output_html.write_text(output)
    print(f"Generated {output_html}")


if __name__ == "__main__":
    main()
