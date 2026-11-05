//! `data/shopping.dat` (CShopping).
//!
//! The file is opened `"r"` by three functions, all reading lines with `LoadLine` (0x536F80)
//! and skipping lines that start with NUL or `#`:
//!
//! * `CShopping::LoadStats` (0x49B6A0), at start-up from `CShopping::Init`: finds `section
//!   prices`, then walks its subsections (`GetNextSection`) and reads every item line;
//! * `CShopping::LoadPrices` (0x49B8D0) and `CShopping::LoadShop` (0x49BBE0), in game when a
//!   shop is entered: find `prices`/`shops` and the named subsection.
//!
//! The structure they share: a line starting with `section` (7 bytes) opens a section whose
//! name is the second `strtok(.., " \t")` token; a line starting with `end` (3 bytes) closes the
//! innermost section; every other line is an item whose fields are the `strtok(.., " \t,")`
//! tokens (`LoadStats`/`LoadPrices`) or `strtok(.., " \t")` tokens (`LoadShop`: `type <name>`,
//! `item <name>`). Because `LoadLine` already turned tabs and commas into spaces, both token
//! sets are the space-separated words. Section names are matched with `_stricmp`.
//!
//! This module keeps that tree with the raw tokens; which tokens each loader consumes
//! (key, name tag, section-specific fields, two stat modifiers, price) is the loaders' business.

use super::{counts, strtok_all};
use crate::data::{DataFile, dump_rec};
use crate::lines::{Mode, load_lines};

/// An item line.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub line: usize,
    pub tokens: Vec<Vec<u8>>,
}

/// A section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Section {
    pub line: usize,
    /// Second token of the `section` line (`None` if there is none).
    pub name: Option<Vec<u8>>,
    pub items: Vec<Item>,
    pub children: Vec<Section>,
    /// Line of the closing `end`.
    pub end_line: Option<usize>,
}

impl Section {
    /// Child section by name (`_stricmp`).
    pub fn child(&self, name: &str) -> Option<&Section> {
        self.children.iter().find(|c| {
            c.name
                .as_deref()
                .is_some_and(|n| super::stricmp_eq(n, name.as_bytes()))
        })
    }
}

/// A parsed `shopping.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shopping {
    /// Top-level sections (`prices`, `shops`).
    pub sections: Vec<Section>,
    /// Item lines outside any section and `end` lines without an open section.
    pub stray: Vec<(usize, Vec<u8>)>,
}

impl Shopping {
    pub fn parse(data: &[u8]) -> Shopping {
        let mut out = Shopping::default();
        let mut stack: Vec<Section> = Vec::new();
        for l in load_lines(data, Mode::Text) {
            let t = &l.text[..];
            let c = l.first();
            if c == 0 || c == b'#' {
                continue;
            }
            if t.starts_with(b"section") {
                let toks = strtok_all(t, b" \t");
                stack.push(Section {
                    line: l.number,
                    name: toks.get(1).map(|n| n.to_vec()),
                    ..Default::default()
                });
            } else if t.starts_with(b"end") {
                match stack.pop() {
                    Some(mut s) => {
                        s.end_line = Some(l.number);
                        match stack.last_mut() {
                            Some(p) => p.children.push(s),
                            None => out.sections.push(s),
                        }
                    }
                    None => out.stray.push((l.number, t.to_vec())),
                }
            } else {
                let item = Item {
                    line: l.number,
                    tokens: strtok_all(t, b" \t,").iter().map(|x| x.to_vec()).collect(),
                };
                match stack.last_mut() {
                    Some(s) => s.items.push(item),
                    None => out.stray.push((l.number, t.to_vec())),
                }
            }
        }
        // Unclosed sections.
        while let Some(s) = stack.pop() {
            match stack.last_mut() {
                Some(p) => p.children.push(s),
                None => out.sections.push(s),
            }
        }
        out
    }

    /// Top-level section by name (`_stricmp`).
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|c| {
            c.name
                .as_deref()
                .is_some_and(|n| super::stricmp_eq(n, name.as_bytes()))
        })
    }
}

fn walk<'a>(s: &'a Section, f: &mut impl FnMut(&'a Section, usize), depth: usize) {
    f(s, depth);
    for c in &s.children {
        walk(c, f, depth + 1);
    }
}

impl DataFile for Shopping {
    fn counts(&self) -> Vec<(String, u64)> {
        let (mut secs, mut items) = (0u64, 0u64);
        for s in &self.sections {
            walk(
                s,
                &mut |s, _| {
                    secs += 1;
                    items += s.items.len() as u64;
                },
                0,
            );
        }
        let price_items = self
            .section("prices")
            .map_or(0, |p| p.children.iter().map(|c| c.items.len() as u64).sum());
        counts(vec![
            ("shopping.dat: sections".into(), secs),
            ("shopping.dat: item lines".into(), items),
            ("shopping.dat: price items".into(), price_items),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for s in &self.sections {
            walk(
                s,
                &mut |s, d| {
                    dump_rec!(
                        &mut o,
                        "section",
                        line = s.line,
                        depth = d,
                        name = s.name,
                        end = s.end_line
                    );
                    for it in &s.items {
                        o.push_str(&format!("item line={}", it.line));
                        for t in &it.tokens {
                            dump_rec!(&mut o, "", tok = t);
                            o.pop();
                        }
                        o.push('\n');
                    }
                },
                0,
            );
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .stray
            .iter()
            .map(|(n, t)| format!("line {n}: outside any section: {:?}", crate::data::lossy(t)))
            .collect();
        for s in &self.sections {
            walk(
                s,
                &mut |s, _| {
                    if s.end_line.is_none() {
                        v.push(format!("line {}: section not closed", s.line));
                    }
                },
                0,
            );
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree() {
        let s = Shopping::parse(
            b"# c\r\nsection prices\r\n\tsection CarMods\r\n\t\tbnt\tBMB\trespect 0\tsexy 0\t150\r\n\tend\t\t\r\nend\r\n\
section shops\r\n\tsection CSchp # BINCO\r\n\ttype Clothes\r\n\titem x\r\n\tend\r\nend\r\n",
        );
        assert_eq!(s.sections.len(), 2);
        let p = s.section("PRICES").unwrap();
        let cm = p.child("carmods").unwrap();
        assert_eq!(cm.items[0].tokens.len(), 7);
        assert_eq!(
            s.section("shops").unwrap().children[0].name.as_deref(),
            Some(&b"CSchp"[..])
        );
        assert!(s.notes().is_empty());
    }
}
