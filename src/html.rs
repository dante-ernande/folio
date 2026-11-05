//! A small, forgiving HTML/XHTML reader that turns chapter markup into
//! simple blocks of styled text.

#[derive(Clone, Debug)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Para,
    Heading(u8),
    Quote,
    Item,
    Pre,
    Rule,
    Image { src: String, alt: String },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub kind: Kind,
    pub spans: Vec<Span>,
}

impl Block {
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
    pub fn words(&self) -> usize {
        self.spans.iter().map(|s| s.text.split_whitespace().count()).sum()
    }
}

#[derive(Debug)]
pub enum Tok {
    Start(String, Vec<(String, String)>, bool),
    End(String),
    Text(String),
}

fn find(s: &str, from: usize, pat: &str) -> Option<usize> {
    s.get(from..)?.find(pat).map(|p| p + from)
}

pub fn tokenize(s: &str) -> Vec<Tok> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut text_start = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        if i > text_start {
            out.push(Tok::Text(s[text_start..i].to_string()));
        }
        let rest = &s[i..];
        if rest.starts_with("<!--") {
            i = find(s, i + 4, "-->").map(|p| p + 3).unwrap_or(b.len());
        } else if rest.starts_with("<![CDATA[") {
            let end = find(s, i + 9, "]]>").unwrap_or(b.len());
            out.push(Tok::Text(s[i + 9..end].to_string()));
            i = (end + 3).min(b.len());
        } else if matches!(b.get(i + 1), Some(b'!') | Some(b'?')) {
            i = find(s, i, ">").map(|p| p + 1).unwrap_or(b.len());
        } else {
            // find the closing '>' while respecting quotes
            let mut j = i + 1;
            let mut quote: u8 = 0;
            while j < b.len() {
                let c = b[j];
                if quote != 0 {
                    if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = c;
                } else if c == b'>' {
                    break;
                }
                j += 1;
            }
            let inner = &s[i + 1..j.min(b.len())];
            if let Some(t) = parse_tag(inner) {
                out.push(t);
            }
            i = (j + 1).min(b.len());
        }
        text_start = i;
    }
    if text_start < b.len() {
        out.push(Tok::Text(s[text_start..].to_string()));
    }
    out
}

fn local(name: &str) -> String {
    let n = name.rsplit(':').next().unwrap_or(name);
    n.to_ascii_lowercase()
}

fn parse_tag(inner: &str) -> Option<Tok> {
    let inner = inner.trim();
    if inner.is_empty() {
        return None;
    }
    if let Some(rest) = inner.strip_prefix('/') {
        let name = rest.split(|c: char| c.is_whitespace()).next().unwrap_or("");
        return Some(Tok::End(local(name)));
    }
    let self_closing = inner.ends_with('/');
    let inner = inner.trim_end_matches('/');
    let name_end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    let name = local(&inner[..name_end]);
    let mut attrs = Vec::new();
    let chars: Vec<char> = inner[name_end..].chars().collect();
    let mut k = 0;
    while k < chars.len() {
        while k < chars.len() && chars[k].is_whitespace() {
            k += 1;
        }
        let ks = k;
        while k < chars.len() && !chars[k].is_whitespace() && chars[k] != '=' {
            k += 1;
        }
        let key: String = chars[ks..k].iter().collect::<String>().to_ascii_lowercase();
        while k < chars.len() && chars[k].is_whitespace() {
            k += 1;
        }
        let mut val = String::new();
        if k < chars.len() && chars[k] == '=' {
            k += 1;
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            if k < chars.len() && (chars[k] == '"' || chars[k] == '\'') {
                let q = chars[k];
                k += 1;
                let vs = k;
                while k < chars.len() && chars[k] != q {
                    k += 1;
                }
                val = chars[vs..k].iter().collect();
                k += 1;
            } else {
                let vs = k;
                while k < chars.len() && !chars[k].is_whitespace() {
                    k += 1;
                }
                val = chars[vs..k].iter().collect();
            }
        }
        if !key.is_empty() {
            attrs.push((key, decode_entities(&val)));
        }
    }
    Some(Tok::Start(name, attrs, self_closing))
}

pub fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        let after = &rest[p + 1..];
        let semi = after.find(';').filter(|&q| q <= 10);
        if let Some(q) = semi {
            let name = &after[..q];
            let rep: Option<String> = if let Some(num) = name.strip_prefix('#') {
                let code = if let Some(hex) = num.strip_prefix('x').or_else(|| num.strip_prefix('X')) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    num.parse::<u32>().ok()
                };
                code.and_then(char::from_u32).map(|c| c.to_string())
            } else {
                named_entity(name).map(|c| c.to_string())
            };
            if let Some(r) = rep {
                out.push_str(&r);
                rest = &after[q + 1..];
                continue;
            }
        }
        out.push('&');
        rest = after;
    }
    out.push_str(rest);
    out
}

fn named_entity(n: &str) -> Option<&'static str> {
    Some(match n {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => "\u{a0}",
        "mdash" => "—",
        "ndash" => "–",
        "hellip" => "…",
        "lsquo" => "‘",
        "rsquo" => "’",
        "ldquo" => "“",
        "rdquo" => "”",
        "laquo" => "«",
        "raquo" => "»",
        "copy" => "©",
        "reg" => "®",
        "trade" => "™",
        "shy" => "",
        "middot" => "·",
        "bull" => "•",
        "deg" => "°",
        "times" => "×",
        "eacute" => "é",
        "egrave" => "è",
        "aacute" => "á",
        "agrave" => "à",
        "iacute" => "í",
        "oacute" => "ó",
        "uacute" => "ú",
        "ntilde" => "ñ",
        "uuml" => "ü",
        "ouml" => "ö",
        "auml" => "ä",
        "ccedil" => "ç",
        "szlig" => "ß",
        "iexcl" => "¡",
        "iquest" => "¿",
        "thinsp" | "ensp" | "emsp" => " ",
        "zwnj" | "zwj" => "",
        _ => return None,
    })
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    spans: Vec<Span>,
    last_space: bool,
    skip: usize,
    bold: usize,
    italic: usize,
    heading: Option<u8>,
    quote: usize,
    item: bool,
    pre: usize,
}

impl Builder {
    fn push_str(&mut self, t: &str) {
        let bold = self.bold > 0 || self.heading.is_some();
        let italic = self.italic > 0;
        match self.spans.last_mut() {
            Some(s) if s.bold == bold && s.italic == italic => s.text.push_str(t),
            _ => self.spans.push(Span { text: t.to_string(), bold, italic }),
        }
    }

    fn text(&mut self, t: &str) {
        if self.skip > 0 {
            return;
        }
        if self.pre > 0 {
            self.push_str(t);
            return;
        }
        let mut buf = String::with_capacity(t.len());
        for c in t.chars() {
            if c.is_whitespace() && c != '\u{a0}' {
                if !self.last_space && !(self.spans.is_empty() && buf.is_empty()) {
                    buf.push(' ');
                    self.last_space = true;
                }
            } else {
                buf.push(c);
                self.last_space = false;
            }
        }
        if !buf.is_empty() {
            self.push_str(&buf);
        }
    }

    fn newline(&mut self) {
        if !self.spans.is_empty() {
            if let Some(s) = self.spans.last_mut() {
                if s.text.ends_with(' ') {
                    s.text.pop();
                }
            }
            self.push_str("\n");
            self.last_space = true;
        }
    }

    fn flush(&mut self) {
        if let Some(s) = self.spans.last_mut() {
            while s.text.ends_with(' ') || s.text.ends_with('\n') {
                s.text.pop();
            }
        }
        let spans: Vec<Span> = std::mem::take(&mut self.spans)
            .into_iter()
            .filter(|s| !s.text.is_empty())
            .collect();
        self.last_space = true;
        let has_text = spans.iter().any(|s| !s.text.trim().is_empty());
        if !has_text {
            return;
        }
        let kind = if let Some(l) = self.heading {
            Kind::Heading(l)
        } else if self.pre > 0 {
            Kind::Pre
        } else if self.item {
            Kind::Item
        } else if self.quote > 0 {
            Kind::Quote
        } else {
            Kind::Para
        };
        self.blocks.push(Block { kind, spans });
    }
}

const BLOCK_TAGS: &[&str] = &[
    "p", "div", "section", "article", "header", "footer", "aside", "figure", "figcaption",
    "table", "tr", "dt", "dd", "ul", "ol", "dl", "nav", "main", "center", "body", "html",
];

pub fn parse_blocks(html: &str) -> Vec<Block> {
    let mut b = Builder { last_space: true, ..Default::default() };
    for t in tokenize(html) {
        match t {
            Tok::Start(name, attrs, sc) => {
                let n = name.as_str();
                match n {
                    "head" | "script" | "style" | "title" => {
                        if !sc {
                            b.skip += 1
                        }
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        b.flush();
                        b.heading = Some(n[1..].parse().unwrap_or(1));
                    }
                    "blockquote" => {
                        b.flush();
                        b.quote += 1;
                    }
                    "li" => {
                        b.flush();
                        b.item = true;
                    }
                    "pre" => {
                        b.flush();
                        b.pre += 1;
                    }
                    "br" => b.newline(),
                    "hr" => {
                        b.flush();
                        b.blocks.push(Block { kind: Kind::Rule, spans: vec![] });
                    }
                    "img" | "image" => {
                        if b.skip == 0 {
                            let src = attr(&attrs, "src")
                                .or_else(|| attr(&attrs, "xlink:href"))
                                .or_else(|| attr(&attrs, "href"))
                                .unwrap_or("")
                                .to_string();
                            let alt = attr(&attrs, "alt").unwrap_or("").to_string();
                            if !src.is_empty() {
                                b.flush();
                                b.blocks.push(Block { kind: Kind::Image { src, alt }, spans: vec![] });
                            }
                        }
                    }
                    "b" | "strong" if !sc => b.bold += 1,
                    "i" | "em" | "cite" | "var" | "dfn" if !sc => b.italic += 1,
                    "td" | "th" => b.text(" "),
                    _ if BLOCK_TAGS.contains(&n) => b.flush(),
                    _ => {}
                }
            }
            Tok::End(name) => {
                let n = name.as_str();
                match n {
                    "head" | "script" | "style" | "title" => b.skip = b.skip.saturating_sub(1),
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        b.flush();
                        b.heading = None;
                    }
                    "blockquote" => {
                        b.flush();
                        b.quote = b.quote.saturating_sub(1);
                    }
                    "li" => {
                        b.flush();
                        b.item = false;
                    }
                    "pre" => {
                        b.flush();
                        b.pre = b.pre.saturating_sub(1);
                    }
                    "b" | "strong" => b.bold = b.bold.saturating_sub(1),
                    "i" | "em" | "cite" | "var" | "dfn" => b.italic = b.italic.saturating_sub(1),
                    _ if BLOCK_TAGS.contains(&n) => b.flush(),
                    _ => {}
                }
            }
            Tok::Text(t) => {
                let d = decode_entities(&t);
                b.text(&d);
            }
        }
    }
    b.flush();
    b.blocks
}

/// Reads the table of contents out of an EPUB 3 navigation document.
/// Returns (href, label, depth).
pub fn nav_toc(html: &str) -> Vec<(String, String, usize)> {
    let toks = tokenize(html);
    let is_toc_nav = |attrs: &[(String, String)]| {
        attr(attrs, "epub:type").map(|v| v.contains("toc")).unwrap_or(false)
            || attr(attrs, "role").map(|v| v.contains("toc")).unwrap_or(false)
    };
    let any_toc = toks
        .iter()
        .any(|t| matches!(t, Tok::Start(n, a, _) if n == "nav" && is_toc_nav(a)));
    let mut out = Vec::new();
    let mut nav_depth = 0usize;
    let mut done = false;
    let mut ol = 0usize;
    let mut cur: Option<(String, String)> = None;
    for t in &toks {
        match t {
            Tok::Start(n, a, _) if n == "nav" => {
                if nav_depth > 0 {
                    nav_depth += 1;
                } else if !done && (!any_toc || is_toc_nav(a)) {
                    nav_depth = 1;
                }
            }
            Tok::End(n) if n == "nav" => {
                if nav_depth > 0 {
                    nav_depth -= 1;
                    if nav_depth == 0 {
                        done = true;
                    }
                }
            }
            _ if nav_depth == 0 => {}
            Tok::Start(n, _, _) if n == "ol" || n == "ul" => ol += 1,
            Tok::End(n) if n == "ol" || n == "ul" => ol = ol.saturating_sub(1),
            Tok::Start(n, a, _) if n == "a" => {
                if let Some(h) = attr(a, "href") {
                    cur = Some((h.to_string(), String::new()));
                }
            }
            Tok::End(n) if n == "a" => {
                if let Some((h, l)) = cur.take() {
                    let label = l.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !label.is_empty() {
                        out.push((h, label, ol.saturating_sub(1)));
                    }
                }
            }
            Tok::Text(s) => {
                if let Some((_, l)) = cur.as_mut() {
                    l.push_str(&decode_entities(s));
                }
            }
            _ => {}
        }
    }
    out
}
