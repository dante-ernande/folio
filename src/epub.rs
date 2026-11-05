//! Opening EPUB files: metadata, chapters, contents and pictures.

use crate::html::{self, Block, Kind};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

pub struct Chapter {
    pub title: String,
    pub blocks: Vec<Block>,
    pub words: usize,
}

impl Chapter {
    pub fn new(title: &str, blocks: Vec<Block>) -> Self {
        let words = blocks.iter().map(|b| b.words()).sum();
        Self { title: title.to_string(), blocks, words }
    }
}

pub struct TocEntry {
    pub label: String,
    pub chapter: usize,
    pub depth: usize,
}

pub struct Book {
    pub title: String,
    pub author: String,
    pub chapters: Vec<Chapter>,
    pub toc: Vec<TocEntry>,
    pub images: HashMap<String, Vec<u8>>,
    pub cover: Option<Vec<u8>>,
}

struct Archive {
    zip: ZipArchive<BufReader<File>>,
    names: HashMap<String, String>,
}

impl Archive {
    fn open(path: &Path) -> Result<Self, String> {
        let f = File::open(path).map_err(|e| format!("Couldn't open the file: {e}"))?;
        let zip = ZipArchive::new(BufReader::new(f)).map_err(|_| "This file isn't a valid EPUB.".to_string())?;
        let names = zip
            .file_names()
            .map(|n| (n.to_lowercase(), n.to_string()))
            .collect();
        Ok(Self { zip, names })
    }

    fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        let key = self.names.get(&name.to_lowercase())?.clone();
        let mut f = self.zip.by_name(&key).ok()?;
        let mut v = Vec::new();
        f.read_to_end(&mut v).ok()?;
        Some(v)
    }

    fn read_str(&mut self, name: &str) -> Option<String> {
        let b = self.read(name)?;
        let s = String::from_utf8_lossy(&b).into_owned();
        Some(s.trim_start_matches('\u{feff}').to_string())
    }
}

struct Item {
    href: String,
    media: String,
    props: String,
}

struct Opf {
    title: String,
    author: String,
    manifest: HashMap<String, Item>,
    spine: Vec<String>,
    ncx: Option<String>,
    nav: Option<String>,
    cover: Option<String>,
}

fn xml_opts() -> roxmltree::ParsingOptions {
    roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz");
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn dir_of(path: &str) -> String {
    match path.rfind('/') {
        Some(p) => path[..p].to_string(),
        None => String::new(),
    }
}

/// Joins `href` onto `base` (a folder inside the archive) and tidies "." and "..".
pub fn resolve(base: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or("");
    let href = percent_decode(href);
    let joined = if let Some(abs) = href.strip_prefix('/') {
        abs.to_string()
    } else if base.is_empty() {
        href
    } else {
        format!("{base}/{href}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

fn node_text(n: roxmltree::Node) -> String {
    let t: String = n.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn read_opf(ar: &mut Archive) -> Result<Opf, String> {
    let container = ar
        .read_str("META-INF/container.xml")
        .ok_or("This file isn't an EPUB (no container).")?;
    let doc = roxmltree::Document::parse_with_options(&container, xml_opts())
        .map_err(|e| format!("Broken EPUB container: {e}"))?;
    let opf_path = doc
        .descendants()
        .find(|n| n.tag_name().name() == "rootfile")
        .and_then(|n| n.attribute("full-path"))
        .ok_or("Broken EPUB: no package file.")?
        .to_string();
    let text = ar.read_str(&opf_path).ok_or("Broken EPUB: package file missing.")?;
    let doc = roxmltree::Document::parse_with_options(&text, xml_opts())
        .map_err(|e| format!("Broken EPUB package: {e}"))?;
    let base = dir_of(&opf_path);

    let title = doc
        .descendants()
        .find(|n| n.tag_name().name() == "title")
        .map(node_text)
        .unwrap_or_default();
    let authors: Vec<String> = doc
        .descendants()
        .filter(|n| n.tag_name().name() == "creator")
        .map(node_text)
        .filter(|s| !s.is_empty())
        .collect();
    let author = authors.join(", ");

    let mut manifest = HashMap::new();
    let mut nav = None;
    let mut cover = None;
    for n in doc.descendants().filter(|n| n.tag_name().name() == "item") {
        let (Some(id), Some(href)) = (n.attribute("id"), n.attribute("href")) else { continue };
        let item = Item {
            href: resolve(&base, href),
            media: n.attribute("media-type").unwrap_or("").to_string(),
            props: n.attribute("properties").unwrap_or("").to_string(),
        };
        if item.props.split_whitespace().any(|p| p == "nav") {
            nav = Some(item.href.clone());
        }
        if item.props.contains("cover-image") {
            cover = Some(item.href.clone());
        }
        manifest.insert(id.to_string(), item);
    }
    if cover.is_none() {
        let meta_id = doc
            .descendants()
            .find(|n| n.tag_name().name() == "meta" && n.attribute("name") == Some("cover"))
            .and_then(|n| n.attribute("content"));
        if let Some(it) = meta_id.and_then(|id| manifest.get(id)) {
            cover = Some(it.href.clone());
        }
    }
    if cover.is_none() {
        cover = manifest
            .iter()
            .find(|(id, it)| it.media.starts_with("image/") && (id.to_lowercase().contains("cover") || it.href.to_lowercase().contains("cover")))
            .map(|(_, it)| it.href.clone());
    }

    let mut spine = Vec::new();
    let mut ncx = None;
    if let Some(sp) = doc.descendants().find(|n| n.tag_name().name() == "spine") {
        if let Some(t) = sp.attribute("toc").and_then(|id| manifest.get(id)) {
            ncx = Some(t.href.clone());
        }
        for r in sp.children().filter(|n| n.tag_name().name() == "itemref") {
            if let Some(id) = r.attribute("idref") {
                spine.push(id.to_string());
            }
        }
    }
    if ncx.is_none() {
        ncx = manifest
            .values()
            .find(|it| it.media.contains("dtbncx"))
            .map(|it| it.href.clone());
    }
    Ok(Opf { title, author, manifest, spine, ncx, nav, cover })
}

fn ncx_toc(text: &str) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    let Ok(doc) = roxmltree::Document::parse_with_options(text, xml_opts()) else {
        return out;
    };
    fn walk(n: roxmltree::Node, depth: usize, out: &mut Vec<(String, String, usize)>) {
        for c in n.children().filter(|c| c.tag_name().name() == "navPoint") {
            let label = c
                .children()
                .find(|x| x.tag_name().name() == "navLabel")
                .map(node_text)
                .unwrap_or_default();
            let src = c
                .children()
                .find(|x| x.tag_name().name() == "content")
                .and_then(|x| x.attribute("src"))
                .unwrap_or("");
            if !label.is_empty() && !src.is_empty() {
                out.push((src.to_string(), label, depth));
            }
            walk(c, depth + 1, out);
        }
    }
    if let Some(map) = doc.descendants().find(|n| n.tag_name().name() == "navMap") {
        walk(map, 0, &mut out);
    }
    out
}

pub fn load(path: &Path) -> Result<Book, String> {
    let mut ar = Archive::open(path)?;
    let opf = read_opf(&mut ar)?;

    let mut chapters: Vec<Chapter> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for idref in &opf.spine {
        let Some(item) = opf.manifest.get(idref) else { continue };
        let lower = item.href.to_lowercase();
        let is_html = item.media.contains("html")
            || lower.ends_with(".html")
            || lower.ends_with(".xhtml")
            || lower.ends_with(".htm");
        if !is_html {
            continue;
        }
        let Some(text) = ar.read_str(&item.href) else { continue };
        let mut blocks = html::parse_blocks(&text);
        let dir = dir_of(&item.href);
        for b in blocks.iter_mut() {
            if let Kind::Image { src, .. } = &mut b.kind {
                *src = resolve(&dir, src);
            }
        }
        index.insert(item.href.clone(), chapters.len());
        chapters.push(Chapter::new("", blocks));
    }
    if chapters.is_empty() {
        return Err("This book has no readable chapters.".into());
    }

    let mut images = HashMap::new();
    for c in &chapters {
        for b in &c.blocks {
            if let Kind::Image { src, .. } = &b.kind {
                if !images.contains_key(src) {
                    if let Some(bytes) = ar.read(src) {
                        images.insert(src.clone(), bytes);
                    }
                }
            }
        }
    }

    let mut raw: Vec<(String, String, usize, String)> = Vec::new();
    if let Some(nav) = &opf.nav {
        if let Some(t) = ar.read_str(nav) {
            let d = dir_of(nav);
            raw = html::nav_toc(&t).into_iter().map(|(h, l, dp)| (h, l, dp, d.clone())).collect();
        }
    }
    if raw.is_empty() {
        if let Some(ncx) = &opf.ncx {
            if let Some(t) = ar.read_str(ncx) {
                let d = dir_of(ncx);
                raw = ncx_toc(&t).into_iter().map(|(h, l, dp)| (h, l, dp, d.clone())).collect();
            }
        }
    }
    let mut toc: Vec<TocEntry> = raw
        .into_iter()
        .filter_map(|(href, label, depth, dir)| {
            let full = resolve(&dir, &href);
            index.get(&full).map(|&chapter| TocEntry { label, chapter, depth })
        })
        .collect();

    for (i, c) in chapters.iter_mut().enumerate() {
        let from_toc = toc.iter().find(|t| t.chapter == i).map(|t| t.label.clone());
        let from_heading = c
            .blocks
            .iter()
            .find(|b| matches!(b.kind, Kind::Heading(_)))
            .map(|b| b.text().replace('\n', " "));
        let mut t = from_toc.or(from_heading).unwrap_or_else(|| format!("Section {}", i + 1));
        if t.chars().count() > 70 {
            t = t.chars().take(68).collect::<String>() + "…";
        }
        c.title = t;
    }
    if toc.is_empty() {
        toc = chapters
            .iter()
            .enumerate()
            .filter(|(_, c)| c.words > 0)
            .map(|(i, c)| TocEntry { label: c.title.clone(), chapter: i, depth: 0 })
            .collect();
    }

    let cover = opf.cover.as_ref().and_then(|h| ar.read(h));
    let title = if opf.title.is_empty() {
        path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Untitled".into())
    } else {
        opf.title
    };
    let author = if opf.author.is_empty() { "Unknown author".into() } else { opf.author };
    Ok(Book { title, author, chapters, toc, images, cover })
}

/// Only the cover picture, for the library shelf.
pub fn load_cover(path: &Path) -> Option<Vec<u8>> {
    let mut ar = Archive::open(path).ok()?;
    let opf = read_opf(&mut ar).ok()?;
    let href = opf.cover?;
    ar.read(&href)
}
