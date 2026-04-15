#!/usr/bin/env python3
"""Build landing/blog.html and landing/blog/<slug>.html from markdown sources.

Reads all *.md files from the top-level `blog/` folder, parses YAML-ish
frontmatter, converts the markdown body to HTML, and writes:

  landing/blog.html              -- index page listing all posts
  landing/blog/<slug>.html       -- one HTML page per post

The generated pages reuse the existing landing-page design (style.css,
nav, fonts) so the blog feels native to the rest of the site.
"""

from __future__ import annotations

import html
import re
from datetime import datetime
from pathlib import Path

# ----------------------------- frontmatter ------------------------------ #


def parse_frontmatter(text: str) -> tuple[dict, str]:
    """Parse a simple YAML-style frontmatter block from the top of `text`.

    Supports key: value pairs only (no nesting, no lists). Values may be
    optionally quoted with single or double quotes.
    """
    if not text.startswith("---"):
        return {}, text

    end = text.find("\n---", 3)
    if end == -1:
        return {}, text

    raw = text[3:end].strip("\n")
    body = text[end + 4 :].lstrip("\n")

    meta: dict = {}
    for line in raw.splitlines():
        line = line.rstrip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            continue
        key, _, value = line.partition(":")
        key = key.strip()
        value = value.strip()
        if (value.startswith('"') and value.endswith('"')) or (
            value.startswith("'") and value.endswith("'")
        ):
            value = value[1:-1]
        meta[key] = value
    return meta, body


# ------------------------------ markdown -------------------------------- #


def _escape(text: str) -> str:
    return html.escape(text, quote=False)


_INLINE_CODE_RE = re.compile(r"`([^`]+)`")
_BOLD_RE = re.compile(r"\*\*([^*]+)\*\*")
_ITALIC_RE = re.compile(r"(?<!\*)\*([^*\n]+)\*(?!\*)")
_IMAGE_RE = re.compile(r"!\[([^\]]*)\]\(([^)]+)\)")
_LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")


def render_inline(text: str) -> str:
    """Convert markdown inline formatting (code, bold, italic, links) to HTML.

    Inline code spans are processed first; their contents are escaped and
    masked out so other inline rules don't touch them.
    """
    placeholders: list[str] = []

    def _stash_code(match: re.Match) -> str:
        placeholders.append(f"<code>{_escape(match.group(1))}</code>")
        return f"\x00{len(placeholders) - 1}\x00"

    text = _INLINE_CODE_RE.sub(_stash_code, text)
    text = _escape(text)

    # Images must run before links so ![alt](src) isn't consumed by [alt](src).
    text = _IMAGE_RE.sub(lambda m: f'<img src="{m.group(2)}" alt="{m.group(1)}">', text)
    # Links must run before bold/italic so the URL isn't mangled.
    text = _LINK_RE.sub(lambda m: f'<a href="{m.group(2)}">{m.group(1)}</a>', text)
    text = _BOLD_RE.sub(r"<strong>\1</strong>", text)
    text = _ITALIC_RE.sub(r"<em>\1</em>", text)

    def _restore(match: re.Match) -> str:
        return placeholders[int(match.group(1))]

    text = re.sub(r"\x00(\d+)\x00", _restore, text)
    return text


def _is_table_separator(line: str) -> bool:
    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        return False
    cells = [c.strip() for c in stripped.strip("|").split("|")]
    if not cells:
        return False
    return all(re.fullmatch(r":?-{3,}:?", c) for c in cells)


def _split_table_row(line: str) -> list[str]:
    return [c.strip() for c in line.strip().strip("|").split("|")]


def render_markdown(md: str) -> str:
    """Convert a markdown body into HTML.

    Supports the subset of markdown actually used in the blog posts:
    headings, paragraphs, fenced code blocks, blockquotes (italic intro),
    unordered lists, GFM-style tables, horizontal rules, and inline
    formatting (code/bold/italic/links).
    """
    lines = md.splitlines()
    out: list[str] = []
    i = 0
    n = len(lines)

    while i < n:
        line = lines[i]

        # Block-level image: a line that is only ![alt](src)
        img_match = re.fullmatch(r"!\[([^\]]*)\]\(([^)]+)\)", line.strip())
        if img_match:
            alt = _escape(img_match.group(1))
            src = img_match.group(2)
            out.append(
                f'<figure class="post-figure"><img src="{src}" alt="{alt}"></figure>'
            )
            i += 1
            continue

        # Fenced code block
        if line.startswith("```"):
            i += 1
            code_lines: list[str] = []
            while i < n and not lines[i].startswith("```"):
                code_lines.append(lines[i])
                i += 1
            i += 1  # consume closing fence
            code_html = _escape("\n".join(code_lines))
            out.append(f'<pre class="post-code"><code>{code_html}</code></pre>')
            continue

        # Headings
        m = re.match(r"^(#{1,6})\s+(.*)$", line)
        if m:
            level = len(m.group(1))
            content = render_inline(m.group(2).strip())
            out.append(f"<h{level}>{content}</h{level}>")
            i += 1
            continue

        # Horizontal rule (---, ***, or a line of dashes)
        if re.fullmatch(r"\s*([-*_])\s*\1\s*\1[\s\1]*", line) or re.fullmatch(
            r"-{3,}", line.strip()
        ):
            out.append('<hr class="post-rule">')
            i += 1
            continue

        # Tables (GFM)
        if (
            line.strip().startswith("|")
            and i + 1 < n
            and _is_table_separator(lines[i + 1])
        ):
            header = _split_table_row(line)
            i += 2  # skip header + separator
            rows: list[list[str]] = []
            while i < n and lines[i].strip().startswith("|"):
                rows.append(_split_table_row(lines[i]))
                i += 1
            out.append('<div class="post-table-wrap"><table class="post-table">')
            out.append("<thead><tr>")
            for cell in header:
                out.append(f"<th>{render_inline(cell)}</th>")
            out.append("</tr></thead>")
            out.append("<tbody>")
            for row in rows:
                out.append("<tr>")
                for cell in row:
                    out.append(f"<td>{render_inline(cell)}</td>")
                out.append("</tr>")
            out.append("</tbody></table></div>")
            continue

        # Unordered list
        if re.match(r"^[-*]\s+", line):
            out.append('<ul class="post-list">')
            while i < n and re.match(r"^[-*]\s+", lines[i]):
                item = re.sub(r"^[-*]\s+", "", lines[i])
                out.append(f"<li>{render_inline(item)}</li>")
                i += 1
            out.append("</ul>")
            continue

        # Blank line
        if not line.strip():
            i += 1
            continue

        # Paragraph: gather consecutive non-empty lines that don't start a
        # different block.
        para: list[str] = [line]
        i += 1
        while i < n and lines[i].strip() and not _starts_block(lines[i], lines, i):
            para.append(lines[i])
            i += 1
        joined = " ".join(s.strip() for s in para)

        # Treat a paragraph that's wrapped entirely in single asterisks as a
        # styled lede / outro.
        if (
            joined.startswith("*")
            and joined.endswith("*")
            and not joined.startswith("**")
        ):
            inner = joined[1:-1]
            out.append(f'<p class="post-lede">{render_inline(inner)}</p>')
        else:
            out.append(f"<p>{render_inline(joined)}</p>")

    return "\n".join(out)


def _starts_block(line: str, all_lines: list[str], idx: int) -> bool:
    """Return True if `line` would start a new block-level construct."""
    if line.startswith("#"):
        return True
    if line.startswith("```"):
        return True
    if re.match(r"^[-*]\s+", line):
        return True
    if line.strip().startswith("|"):
        # A real table starts with a header line followed by a separator.
        if idx + 1 < len(all_lines) and _is_table_separator(all_lines[idx + 1]):
            return True
    if re.fullmatch(r"-{3,}", line.strip()):
        return True
    return False


# ------------------------------- dates ---------------------------------- #


def parse_date(value: str | None) -> datetime | None:
    if not value:
        return None
    try:
        return datetime.strptime(value, "%Y-%m-%d")
    except ValueError:
        return None


def format_date_long(dt: datetime) -> str:
    return dt.strftime("%B %-d, %Y")


def format_date_short(dt: datetime) -> str:
    return dt.strftime("%b %-d, %Y")


# ----------------------------- post model ------------------------------- #


class Post:
    def __init__(self, source: Path):
        text = source.read_text()
        meta, body = parse_frontmatter(text)
        self.source = source
        self.title = meta.get("title", source.stem)
        self.description = meta.get("description", "")
        self.author = meta.get("author", "")
        self.date = parse_date(meta.get("date"))
        self.slug = meta.get("slug") or _slug_from_filename(source.stem)
        self.body_md = body
        self._body_html: str | None = None

    @property
    def body_html(self) -> str:
        if self._body_html is None:
            self._body_html = render_markdown(self.body_md)
        return self._body_html

    @property
    def output_path(self) -> str:
        return f"blog/{self.slug}.html"

    @property
    def date_iso(self) -> str:
        return self.date.strftime("%Y-%m-%d") if self.date else ""

    @property
    def date_long(self) -> str:
        return format_date_long(self.date) if self.date else ""

    @property
    def date_short(self) -> str:
        return format_date_short(self.date) if self.date else ""


def _slug_from_filename(stem: str) -> str:
    # Strip a leading YYYY-MM-DD- prefix if present
    m = re.match(r"\d{4}-\d{2}-\d{2}[-_](.*)", stem)
    return m.group(1) if m else stem


# ----------------------------- shared HTML ------------------------------ #


GH_SVG = (
    '<svg width="14" height="14" fill="currentColor" viewBox="0 0 16 16">'
    '<path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 '
    "0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 "
    "1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 "
    "0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 "
    "2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 "
    "1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 "
    '2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>'
)

FONT_LINK = (
    '<link href="https://fonts.googleapis.com/css2?'
    "family=Playfair+Display:ital,wght@0,400;0,700;0,900;1,400;1,700"
    "&family=DM+Mono:wght@300;400;500"
    "&family=Source+Serif+4:ital,opsz,wght@0,8..60,300;0,8..60,400;0,8..60,600;1,8..60,300;1,8..60,400"
    '&display=swap" rel="stylesheet">'
)


def nav_html(active: str, depth: int = 0) -> str:
    """Render the shared <nav> bar.

    `depth` is how many directories deep the page lives relative to the
    landing root, so we can rewrite relative links to ../ when needed.
    """
    prefix = "../" * depth
    items = [
        ("Home", f"{prefix}index.html"),
        ("Docs", f"{prefix}docs.html"),
        ("Blog", f"{prefix}blog.html"),
        ("Changelog", f"{prefix}changelog.html"),
    ]

    nav_links = []
    mobile_links = []
    for label, href in items:
        cls = ' class="active"' if label.lower() == active.lower() else ""
        mcls = " active" if label.lower() == active.lower() else ""
        nav_links.append(f'    <a href="{href}"{cls}>{label}</a>')
        mobile_links.append(
            f'  <a href="{href}" class="mobile-menu-link{mcls}">{label}</a>'
        )

    nav_links_html = "\n".join(nav_links)
    mobile_links_html = "\n".join(mobile_links)

    return f"""<!-- NAV -->
<nav>
  <a href="{prefix}index.html" class="nav-brand">yaak</a>
  <button class="hamburger" id="hamburger" aria-label="Open menu" aria-expanded="false">
    <span></span><span></span><span></span>
  </button>
  <div class="nav-right">
{nav_links_html}
    <a href="https://github.com/hanneshapke/yaak" class="nav-gh">
      {GH_SVG}
      Star on GitHub
    </a>
  </div>
</nav>

<!-- MOBILE MENU OVERLAY -->
<div class="mobile-menu" id="mobileMenu">
{mobile_links_html}
  <a href="https://github.com/hanneshapke/yaak" class="mobile-menu-link nav-gh">
    {GH_SVG}
    Star on GitHub
  </a>
</div>"""


FOOTER_HTML = """<!-- FOOTER -->
<footer>
  <span>yaak &mdash; open source, Apache-2.0 licensed</span>
  <span>Built with Rust &amp; <a href="https://github.com/hanneshapke/yaak">your favorite LLM</a></span>
</footer>"""


HAMBURGER_SCRIPT = """<script>
const hamburger = document.getElementById('hamburger');
const mobileMenu = document.getElementById('mobileMenu');

hamburger.addEventListener('click', () => {
  const isOpen = mobileMenu.classList.toggle('open');
  hamburger.classList.toggle('active');
  hamburger.setAttribute('aria-expanded', isOpen);
  document.body.style.overflow = isOpen ? 'hidden' : '';
});

mobileMenu.querySelectorAll('.mobile-menu-link').forEach(link => {
  link.addEventListener('click', () => {
    mobileMenu.classList.remove('open');
    hamburger.classList.remove('active');
    hamburger.setAttribute('aria-expanded', 'false');
    document.body.style.overflow = '';
  });
});
</script>"""


# ----------------------------- shared styles ---------------------------- #


SHARED_STYLES = """\
  /* ============ HERO ============ */
  .page-hero {
    padding: 80px 32px 48px;
    border-bottom: 3px double var(--rule);
  }
  .page-hero-inner {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: end;
    gap: 32px;
  }
  .page-kicker {
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 3px;
    color: var(--red);
    margin-bottom: 16px;
    animation: fadeIn 0.5s ease both;
  }
  .page-title {
    font-family: 'Playfair Display', serif;
    font-weight: 900;
    font-size: clamp(40px, 7vw, 72px);
    line-height: 0.95;
    letter-spacing: -2px;
    animation: fadeIn 0.5s ease 0.05s both;
  }
  .page-title em { font-style: italic; font-weight: 400; color: var(--red); }
  .page-meta {
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    color: var(--ink-faint);
    text-align: right;
    letter-spacing: 0.5px;
    line-height: 1.8;
    animation: fadeIn 0.5s ease 0.1s both;
  }
"""


INDEX_STYLES = (
    SHARED_STYLES
    + """
  /* ============ BLOG INDEX ============ */
  .blog-list {
    padding: 56px 32px 100px;
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .post-card {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: 32px;
    padding: 40px 0;
    border-bottom: 1px solid var(--rule);
    text-decoration: none;
    color: inherit;
    animation: fadeIn 0.5s ease both;
    transition: background 0.25s;
  }
  .post-card:first-child { padding-top: 0; }
  .post-card:hover { background: var(--cream); }
  .post-card:hover .post-card-title { color: var(--red); }

  .post-card-meta {
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--ink-faint);
    line-height: 1.7;
    padding-top: 6px;
  }
  .post-card-meta .post-date { display: block; color: var(--red); }
  .post-card-meta .post-author { display: block; margin-top: 6px; }

  .post-card-title {
    font-family: 'Playfair Display', serif;
    font-size: clamp(22px, 3vw, 32px);
    font-weight: 900;
    line-height: 1.15;
    letter-spacing: -0.5px;
    margin-bottom: 12px;
    transition: color 0.2s;
  }
  .post-card-title em { font-style: italic; font-weight: 400; color: var(--red); }

  .post-card-desc {
    font-size: 15px;
    line-height: 1.65;
    color: var(--ink-dim);
    font-weight: 300;
    max-width: 640px;
    margin-bottom: 14px;
  }
  .post-card-more {
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .post-card-more::after { content: '\\2192'; transition: transform 0.2s; }
  .post-card:hover .post-card-more::after { transform: translateX(4px); }

  .blog-empty {
    padding: 80px 32px;
    text-align: center;
    color: var(--ink-faint);
    font-family: 'DM Mono', monospace;
    font-size: 13px;
    letter-spacing: 1px;
  }

  .post-card:nth-child(1) { animation-delay: 0.15s; }
  .post-card:nth-child(2) { animation-delay: 0.22s; }
  .post-card:nth-child(3) { animation-delay: 0.29s; }
  .post-card:nth-child(4) { animation-delay: 0.36s; }
  .post-card:nth-child(5) { animation-delay: 0.43s; }

  @media (max-width: 768px) {
    .blog-list { padding: 32px 16px 60px; }
    .post-card { grid-template-columns: 1fr; gap: 12px; padding: 28px 0; }
    .post-card-meta { padding-top: 0; }
    .page-hero-inner { grid-template-columns: 1fr; }
    .page-meta { text-align: left; }
  }
"""
)


POST_STYLES = (
    SHARED_STYLES
    + """
  .post-meta-row {
    display: flex;
    gap: 16px;
    align-items: center;
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--ink-faint);
    margin-bottom: 16px;
  }
  .post-meta-row .post-date { color: var(--red); }
  .post-meta-row .sep { color: var(--rule); }

  .post-layout {
    padding: 56px 32px 100px;
    display: grid;
    grid-template-columns: minmax(0, 720px);
    justify-content: center;
  }

  .post-content {
    font-family: 'Source Serif 4', Georgia, serif;
    font-size: 17px;
    line-height: 1.75;
    color: var(--ink-light);
    font-weight: 300;
    animation: fadeIn 0.5s ease 0.15s both;
  }
  .post-content > * { max-width: 100%; }

  .post-content p { margin: 0 0 22px; }
  .post-content p.post-lede {
    font-style: italic;
    font-size: 19px;
    color: var(--ink-dim);
    border-left: 3px solid var(--red);
    padding-left: 20px;
    margin-bottom: 30px;
  }

  .post-content h2 {
    font-family: 'Playfair Display', serif;
    font-size: 30px;
    font-weight: 900;
    color: var(--ink);
    letter-spacing: -0.5px;
    margin: 56px 0 18px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--rule);
  }
  .post-content h2 em { font-style: italic; font-weight: 400; color: var(--red); }

  .post-content h3 {
    font-family: 'Playfair Display', serif;
    font-size: 22px;
    font-weight: 700;
    color: var(--ink);
    letter-spacing: -0.3px;
    margin: 36px 0 12px;
  }

  .post-content h4 {
    font-family: 'DM Mono', monospace;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 2px;
    color: var(--red);
    margin: 28px 0 10px;
  }

  .post-content a {
    color: var(--ink);
    text-decoration: none;
    border-bottom: 1px solid var(--red);
    transition: color 0.2s;
  }
  .post-content a:hover { color: var(--red); }

  .post-content strong { color: var(--ink); font-weight: 600; }

  .post-content code {
    font-family: 'DM Mono', monospace;
    font-size: 14px;
    background: var(--code-bg);
    color: var(--code-green);
    padding: 2px 7px;
    border-radius: 3px;
  }

  .post-content pre.post-code {
    font-family: 'DM Mono', monospace;
    font-size: 13px;
    background: var(--code-bg);
    color: var(--code-fg);
    padding: 20px 22px;
    border-radius: 6px;
    line-height: 1.7;
    overflow-x: auto;
    margin: 0 0 26px;
  }
  .post-content pre.post-code code {
    background: none;
    color: inherit;
    padding: 0;
    font-size: 13px;
  }

  .post-content ul.post-list {
    list-style: none;
    padding: 0;
    margin: 0 0 26px;
  }
  .post-content ul.post-list li {
    position: relative;
    padding-left: 24px;
    margin-bottom: 10px;
  }
  .post-content ul.post-list li::before {
    content: '\\203a';
    position: absolute;
    left: 6px;
    top: -2px;
    color: var(--red);
    font-family: 'Playfair Display', serif;
    font-size: 18px;
    line-height: 1.55;
  }

  .post-content hr.post-rule {
    border: none;
    border-top: 3px double var(--rule);
    margin: 40px 0;
  }

  .post-content .post-table-wrap { overflow-x: auto; margin: 0 0 28px; }
  .post-content table.post-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
  }
  .post-content table.post-table th {
    font-family: 'DM Mono', monospace;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--ink-faint);
    text-align: left;
    padding: 12px 14px;
    border-bottom: 2px solid var(--rule);
    white-space: nowrap;
  }
  .post-content table.post-table td {
    padding: 12px 14px;
    border-bottom: 1px solid rgba(209, 201, 192, 0.4);
    color: var(--ink-light);
    font-weight: 300;
    vertical-align: top;
  }
  .post-content table.post-table tr:hover td { background: var(--cream); }

  .back-link {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-family: 'DM Mono', monospace;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    color: var(--ink-dim);
    text-decoration: none;
    margin-bottom: 32px;
    transition: color 0.2s;
  }
  .back-link::before { content: '\\2190'; }
  .back-link:hover { color: var(--red); }

  @media (max-width: 768px) {
    .post-layout { padding: 32px 16px 60px; }
    .post-content { font-size: 16px; }
    .post-content h2 { font-size: 24px; margin-top: 40px; }
    .post-content h3 { font-size: 19px; }
    .page-hero-inner { grid-template-columns: 1fr; }
    .page-meta { text-align: left; }
  }
"""
)


# ----------------------------- index page ------------------------------- #


def render_index(posts: list[Post]) -> str:
    if not posts:
        cards_html = '<div class="blog-empty">No posts yet. Check back soon.</div>'
    else:
        cards = []
        for p in posts:
            meta_html = ""
            if p.date_short:
                meta_html += f'<span class="post-date">{p.date_short}</span>'
            if p.author:
                meta_html += f'<span class="post-author">{html.escape(p.author)}</span>'
            desc_html = (
                f'<p class="post-card-desc">{render_inline(p.description)}</p>'
                if p.description
                else ""
            )
            cards.append(
                f'      <a class="post-card" href="{p.output_path}">\n'
                f'        <div class="post-card-meta">{meta_html}</div>\n'
                f"        <div>\n"
                f'          <h2 class="post-card-title">{render_inline(p.title)}</h2>\n'
                f"          {desc_html}\n"
                f'          <span class="post-card-more">Read post</span>\n'
                f"        </div>\n"
                f"      </a>"
            )
        cards_html = "\n".join(cards)

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>yaak &mdash; blog</title>
{FONT_LINK}
<link rel="stylesheet" href="style.css">
<style>
{INDEX_STYLES}</style>
</head>
<body>

{nav_html("Blog", depth=0)}

<!-- PAGE HERO -->
<section class="page-hero">
  <div class="container">
    <div class="page-hero-inner">
      <div>
        <div class="page-kicker">Field notes</div>
        <h1 class="page-title">The <em>Blog</em></h1>
      </div>
      <div class="page-meta">
        Notes on the CLI<br>
        Updates &amp; ideas
      </div>
    </div>
  </div>
</section>

<!-- POSTS -->
<div class="container">
  <div class="blog-list">
{cards_html}
  </div>
</div>

{FOOTER_HTML}

{HAMBURGER_SCRIPT}

</body>
</html>
"""


# ------------------------------- post page ------------------------------ #


def render_post(post: Post) -> str:
    meta_parts: list[str] = []
    if post.date_long:
        meta_parts.append(f'<span class="post-date">{post.date_long}</span>')
    if post.author:
        meta_parts.append(
            f'<span class="post-author">{html.escape(post.author)}</span>'
        )
    meta_html = '<span class="sep">&middot;</span>'.join(meta_parts)

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{html.escape(post.title)} &mdash; yaak blog</title>
<meta name="description" content="{html.escape(post.description)}">
{FONT_LINK}
<link rel="stylesheet" href="../style.css">
<style>
{POST_STYLES}</style>
</head>
<body>

{nav_html("Blog", depth=1)}

<!-- PAGE HERO -->
<section class="page-hero">
  <div class="container">
    <a href="../blog.html" class="back-link">All posts</a>
    <div class="post-meta-row">
      {meta_html}
    </div>
    <h1 class="page-title">{render_inline(post.title)}</h1>
  </div>
</section>

<!-- POST -->
<div class="container">
  <div class="post-layout">
    <article class="post-content">
{post.body_html}
    </article>
  </div>
</div>

{FOOTER_HTML}

{HAMBURGER_SCRIPT}

</body>
</html>
"""


# --------------------------------- main --------------------------------- #


def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    blog_src = repo_root / "blog"
    landing = repo_root / "landing"
    posts_out_dir = landing / "blog"

    posts_out_dir.mkdir(parents=True, exist_ok=True)

    if not blog_src.exists():
        print(f"No blog source folder at {blog_src}; nothing to build.")
        return

    sources = sorted(blog_src.glob("*.md"))
    posts = [Post(p) for p in sources]
    # Sort newest first; posts without a date sink to the bottom.
    posts.sort(key=lambda p: p.date or datetime.min, reverse=True)

    # Index page
    index_path = landing / "blog.html"
    index_path.write_text(render_index(posts))
    print(f"Generated {index_path}")

    # Per-post pages
    for post in posts:
        out = posts_out_dir / f"{post.slug}.html"
        out.write_text(render_post(post))
        print(f"Generated {out}")


if __name__ == "__main__":
    main()
