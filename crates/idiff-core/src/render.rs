//! A receipt as terminal text, Markdown or one self-contained HTML file.
//!
//! Changes nobody could attribute are always shown in their own section: hiding them would hide
//! them silently. Windows' own changes are counted on the receipt and listed with `all`.

use crate::copy;
use crate::verdict::{Level, Line, Receipt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextOptions {
    /// ANSI colours. The CLI turns them on for a terminal when `NO_COLOR` isn't set.
    pub color: bool,
    /// Also list Windows' own changes. Unknown changes don't wait for this: spec D8 says nothing
    /// is hidden silently, and they aren't counted anywhere else.
    pub all: bool,
    /// Under each line, the items it's about and why each counted.
    pub why: bool,
}

fn icon(level: Level) -> &'static str {
    match level {
        Level::Good => "✓",
        Level::Note => "·",
        Level::Warn => "⚠",
        Level::Bad => "✗",
    }
}

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

fn ansi(level: Level) -> &'static str {
    match level {
        Level::Good => "\x1b[32m",
        Level::Note => DIM,
        Level::Warn => "\x1b[33m",
        Level::Bad => "\x1b[31m",
    }
}

fn footer() -> String {
    copy::FOOTER.replace("{version}", env!("CARGO_PKG_VERSION"))
}

pub fn text(r: &Receipt, o: &TextOptions) -> String {
    let paint = |code: &str, s: &str| {
        if o.color {
            format!("{code}{s}{RESET}")
        } else {
            s.to_string()
        }
    };
    let mut out = format!("{}\n", paint(BOLD, &r.title));
    let add = |lines: &[Line], out: &mut String| {
        for line in lines {
            let row = format!("{} {}", icon(line.level), line.text);
            out.push_str(&paint(ansi(line.level), &row));
            out.push('\n');
            if o.why {
                for (item, why) in line.items.iter().zip(&line.why) {
                    out.push_str(&paint(DIM, &format!("    {item}: {why}")));
                    out.push('\n');
                }
            }
        }
    };
    add(&r.lines, &mut out);
    let section = |heading: &str, lines: &[Line], out: &mut String| {
        if !lines.is_empty() {
            out.push('\n');
            out.push_str(&paint(BOLD, heading));
            out.push('\n');
            add(lines, out);
        }
    };
    section(copy::ALSO_CHANGED, &r.unknown, &mut out);
    if o.all {
        section(copy::HIDDEN_HEADING, &r.hidden, &mut out);
    }
    out
}

/// Escapes what Markdown would read as syntax (`\ | [ ] ( ) * _ < >` and backticks), so a
/// label can't become a link, a table cell or HTML. Line breaks become spaces.
pub fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '|' | '[' | ']' | '(' | ')' | '*' | '_' | '<' | '>' | '`' => {
                out.push('\\');
                out.push(c);
            }
            '\r' => {}
            '\n' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

pub fn markdown(r: &Receipt, all: bool) -> String {
    markdown_receipts(&[r], all)
}

/// Several receipts in one Markdown file (an install, then its leftovers), with one footer.
pub fn markdown_receipts(receipts: &[&Receipt], all: bool) -> String {
    let list = |lines: &[Line]| -> String {
        lines
            .iter()
            .map(|l| format!("- {} {}\n", icon(l.level), md_escape(&l.text)))
            .collect()
    };
    let mut out = String::new();
    for (i, r) in receipts.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("## {}\n\n{}", md_escape(&r.title), list(&r.lines)));
        let mut section = |heading: &str, lines: &[Line]| {
            if !lines.is_empty() {
                out.push_str(&format!("\n### {}\n\n{}", md_escape(heading), list(lines)));
            }
        };
        section(copy::ALSO_CHANGED, &r.unknown);
        if all {
            section(copy::HIDDEN_HEADING, &r.hidden);
        }
    }
    out.push_str(&format!("\n---\n\n{}\n", md_escape(&footer())));
    out
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Monospace, one narrow column and dashed rules, like a till receipt, in light and dark. No
/// fonts, images, scripts or links: the file shows the same anywhere, offline.
const CSS: &str = "
:root {
  color-scheme: light dark;
  --paper: #fbfaf6; --ink: #1d1c1a; --soft: #5d5a53; --rule: #c9c6bd;
  --good: #1f6a45; --warn: #a3410f; --bad: #b3261e;
}
@media (prefers-color-scheme: dark) {
  :root {
    --paper: #1d1c1a; --ink: #ecebe6; --soft: #a9a59b; --rule: #4a4843;
    --good: #7cc79a; --warn: #f0a070; --bad: #ff8a80;
  }
}
* { box-sizing: border-box; }
body { margin: 0; padding: 2rem 1rem; background: var(--paper); color: var(--ink); }
main {
  max-width: 34rem; margin: 0 auto;
  font: 14px/1.6 ui-monospace, 'Cascadia Mono', Consolas, 'SF Mono', Menlo, monospace;
  overflow-wrap: anywhere;
}
h1 { font-size: 1.15em; margin: 0; }
h2 { font-size: 0.85em; font-weight: 600; letter-spacing: 0.12em; text-transform: uppercase; margin: 1.6em 0 0.4em; }
hr { border: 0; border-top: 1px dashed var(--rule); margin: 1em 0; }
ul { list-style: none; margin: 0; padding: 0; }
li { display: grid; grid-template-columns: 2ch 1fr; padding: 0.15em 0; }
.good .icon { color: var(--good); }
.warn { color: var(--warn); }
.bad { color: var(--bad); font-weight: 600; }
.note { color: var(--soft); }
footer { color: var(--soft); font-size: 0.85em; text-align: center; }
.sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
";

pub fn html(r: &Receipt) -> String {
    html_receipts(&[r])
}

/// Several receipts in one HTML file (an install, then its leftovers), titled after the first.
pub fn html_receipts(receipts: &[&Receipt]) -> String {
    let list = |lines: &[Line]| -> String {
        let rows: String = lines
            .iter()
            .map(|l| {
                let (class, said) = match l.level {
                    Level::Good => ("good", copy::LEVEL_GOOD),
                    Level::Note => ("note", copy::LEVEL_NOTE),
                    Level::Warn => ("warn", copy::LEVEL_WARN),
                    Level::Bad => ("bad", copy::LEVEL_BAD),
                };
                // The icon is for the eye; screen readers get the level as a word.
                format!(
                    "<li class=\"{class}\"><span class=\"icon\" aria-hidden=\"true\">{}</span>\
                     <span><span class=\"sr\">{said} </span>{}</span></li>\n",
                    icon(l.level),
                    html_escape(&l.text)
                )
            })
            .collect();
        format!("<ul>\n{rows}</ul>\n")
    };
    let mut body = String::new();
    for (i, r) in receipts.iter().enumerate() {
        if i > 0 {
            body.push_str("<hr>\n");
        }
        body.push_str(&format!("<h1>{}</h1>\n<hr>\n{}", html_escape(&r.title), list(&r.lines)));
        for (heading, lines) in [(copy::ALSO_CHANGED, &r.unknown), (copy::HIDDEN_HEADING, &r.hidden)] {
            if !lines.is_empty() {
                body.push_str(&format!("<h2>{}</h2>\n{}", html_escape(heading), list(lines)));
            }
        }
    }
    let title = receipts.first().map(|r| r.title.as_str()).unwrap_or_default();
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n<style>{CSS}</style>\n</head>\n<body>\n<main>\n{body}<hr>\n\
         <footer>{footer}</footer>\n</main>\n</body>\n</html>\n",
        title = html_escape(title),
        footer = html_escape(&footer()),
    )
}
