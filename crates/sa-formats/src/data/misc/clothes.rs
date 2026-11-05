//! `data/clothes.dat`, as `CClothes::LoadClothesFile` (0x5A7B30) reads it.
//!
//! Opened `"r"`, `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped. Outside a
//! rule block a line starting with `rule` (4 bytes) opens one; other lines are ignored. Inside,
//! a line starting with `end` (3 bytes) closes the block; every other line is split with
//! `strtok(.., " \t,")`:
//!
//! 1. the first token is compared with `_stricmp` against `cuts`=0, `setc`=1, `tex`=2,
//!    `hide`=3, `ignore`=5, `endignore`=4, `exclusive`=7, `endexclusive`=6; **an unknown keyword
//!    leaves the rule type variable unchanged** (it is uninitialised before the first rule), and
//!    the type is appended to the rule list either way;
//! 2. two more tokens are always taken; `setc` takes two further tokens;
//! 3. arguments are hashed with `CKeyGen::GetUppercaseKey`, or for the 2nd argument of
//!    `setc`/`hide` mapped by FUN_005A7A20 (body part name); a `setc` 3rd/4th argument of `-`
//!    (compared over 2 bytes, so exactly `-`) stores 0.
//!
//! Missing tokens would pass NULL to the hash functions (not in the install).

use super::{counts, stricmp_eq, strtok_all};
use crate::data::{DataFile, dump_rec};
use crate::lines::{Mode, load_lines};

/// Keyword table (name, rule type), in comparison order.
pub const KEYWORDS: [(&str, u32); 8] = [
    ("cuts", 0),
    ("setc", 1),
    ("tex", 2),
    ("hide", 3),
    ("ignore", 5),
    ("endignore", 4),
    ("exclusive", 7),
    ("endexclusive", 6),
];

/// One rule line.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub line: usize,
    /// All `strtok` tokens of the line (the exe uses the first 3, or 5 for `setc`).
    pub tokens: Vec<Vec<u8>>,
    /// Type set by this line's keyword (`None` for an unknown keyword).
    pub keyword_type: Option<u32>,
    /// Type the exe stores: this line's, or the previous line's when the keyword is unknown
    /// (`None` = uninitialised stack value).
    pub effective_type: Option<u32>,
}

/// A parsed `clothes.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clothes {
    pub rules: Vec<Rule>,
    /// Number of `rule` blocks.
    pub blocks: usize,
}

impl Clothes {
    pub fn parse(data: &[u8]) -> Clothes {
        let mut out = Clothes::default();
        let mut in_rule = false;
        let mut current: Option<u32> = None;
        for l in load_lines(data, Mode::Text) {
            let t = &l.text[..];
            let c = l.first();
            if c == b'#' || c == 0 {
                continue;
            }
            if !in_rule {
                if t.starts_with(b"rule") {
                    in_rule = true;
                    out.blocks += 1;
                }
                continue;
            }
            if t.starts_with(b"end") {
                in_rule = false;
                continue;
            }
            let toks = strtok_all(t, b" \t,");
            let Some(first) = toks.first() else { continue };
            let kw = KEYWORDS
                .iter()
                .find(|(n, _)| stricmp_eq(first, n.as_bytes()))
                .map(|(_, t)| *t);
            if kw.is_some() {
                current = kw;
            }
            out.rules.push(Rule {
                line: l.number,
                tokens: toks.iter().map(|t| t.to_vec()).collect(),
                keyword_type: kw,
                effective_type: current,
            });
        }
        out
    }
}

impl DataFile for Clothes {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("clothes.dat: rule blocks".into(), self.blocks as u64),
            ("clothes.dat: rules".into(), self.rules.len() as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for r in &self.rules {
            o.push_str(&format!(
                "rule line={} type={:?} effective={:?}",
                r.line, r.keyword_type, r.effective_type
            ));
            for t in &r.tokens {
                dump_rec!(&mut o, "", tok = t);
                o.pop();
            }
            o.push('\n');
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for r in &self.rules {
            if r.keyword_type.is_none() {
                v.push(format!(
                    "line {}: unknown keyword, rule type of the previous line reused",
                    r.line
                ));
            }
            // Keyword + arguments the exe actually uses: cuts/tex 2, setc 4, hide 2,
            // (end)ignore/(end)exclusive 1 (the extra `strtok` result is unused).
            let need = match r.effective_type {
                Some(1) => 5,
                Some(0 | 2 | 3) => 3,
                _ => 2,
            };
            if r.tokens.len() < need {
                v.push(format!(
                    "line {}: {} tokens, the exe reads {need}",
                    r.line,
                    r.tokens.len()
                ));
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        let c = Clothes::parse(
            b"SETC x head\r\nrule\r\nSETC\tgimpleg\thead\tgimpmask\t-\r\nHIDE g legs\r\nfoo a b\r\nend\r\n",
        );
        assert_eq!(c.blocks, 1);
        assert_eq!(c.rules.len(), 3);
        assert_eq!(c.rules[0].keyword_type, Some(1));
        assert_eq!(c.rules[0].tokens.len(), 5);
        assert_eq!(c.rules[2].keyword_type, None);
        assert_eq!(c.rules[2].effective_type, Some(3));
    }
}
