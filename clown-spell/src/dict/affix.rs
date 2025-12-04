use crate::dict::error::DictionaryBuildError;
use regex::Regex;
use std::iter::Peekable;
use std::{
    borrow::Cow,
    collections::HashMap,
    io::{BufRead, Error},
};
#[derive(Debug, PartialEq, Eq)]
enum AffixType {
    Prefix,
    Suffix,
}

#[derive(Debug)]
struct AffixRule {
    kind: AffixType,
    flag: String,
    strip: Option<String>,
    add: String,
    cond: Option<Regex>,
}

impl AffixRule {
    pub fn can_apply(&self, word: &str) -> bool {
        if !self.cond.as_ref().is_some_and(|r| r.is_match(word)) {
            return false;
        }
        if let Some(strip) = self.strip.as_ref() {
            if self.kind == AffixType::Prefix {
                word.starts_with(strip)
            } else {
                word.ends_with(strip)
            }
        } else {
            true
        }
    }

    pub fn apply_rule(&self, word: &str) -> String {
        let mut new_word = word.to_string();
        if let Some(stip) = self.strip.as_ref() {
            if self.kind == AffixType::Prefix {
                new_word.replace_range(..stip.len(), &self.add);
            } else {
                let start = new_word.len() - stip.len();
                new_word.replace_range(start.., &self.add);
            }
        } else if self.kind == AffixType::Prefix {
            new_word.insert_str(0, &self.add);
        } else if self.kind == AffixType::Suffix {
            new_word.push_str(&self.add);
        }

        new_word
    }
}

#[derive(Debug)]
pub struct DictAffix {
    rules: HashMap<String, Vec<AffixRule>>,
    map: HashMap<char, char>,
}

impl DictAffix {
    pub fn try_build<T: BufRead>(reader: T) -> Result<Self, DictionaryBuildError> {
        let mut lines = reader.lines().peekable();
        let mut map: HashMap<char, char> = HashMap::new();
        let mut rules: HashMap<String, Vec<AffixRule>> = HashMap::new();

        while let Some(Ok(line)) = lines.peek() {
            if line.is_empty() {
                lines.next();
                continue;
            }

            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("MAP") => map = DictAffix::parse_map(&mut lines)?,
                Some("SFX") | Some("PFX") => rules = DictAffix::parse_prefix_suffix(&mut lines)?,
                _ => {}
            }

            lines.next();
        }

        Ok(Self { rules, map })
    }

    pub fn transform_word<'a>(&self, word: &'a str) -> Cow<'a, str> {
        let mut out = String::new();

        for c in word.chars() {
            out.push(*self.map.get(&c).unwrap_or(&c));
        }

        if out == word {
            Cow::Borrowed(word)
        } else {
            Cow::Owned(out)
        }
    }

    //Apply affix rules, and return words
    pub fn apply_rules<'a>(
        &'a self,
        word: &'a str,
        flag: &'a str,
    ) -> impl Iterator<Item = String> + 'a {
        self.rules.get(flag).into_iter().flat_map(move |rules| {
            rules.iter().filter_map(move |rule| {
                if rule.can_apply(word) {
                    Some(rule.apply_rule(word))
                } else {
                    None
                }
            })
        })
    }

    //PFX L' a l'A a
    // prefix <flag> <strip> <add> <condition>
    fn parse_prefix_suffix(
        lines: &mut Peekable<impl Iterator<Item = Result<String, Error>>>,
    ) -> Result<HashMap<String, Vec<AffixRule>>, DictionaryBuildError> {
        let mut rules: HashMap<String, Vec<AffixRule>> = HashMap::new();

        //Stop when cannot read a line
        while let Some(Ok(line)) = lines.peek() {
            if line.is_empty() {
                lines.next();
                continue;
            }
            let mut parts = line.split_whitespace();
            if let Some(first) = parts.next() {
                match first {
                    "PFX" | "SFX" => {
                        let flag = parts.next().unwrap_or_default();
                        let strip = parts.next().unwrap_or_default();
                        let add = parts //SFX vC mettre mettre/n'q'd'l'm't's' mettre |||  n'q'd'l'm't's' is a morphological rule
                            .next()
                            .unwrap_or_default()
                            .split('/')
                            .next()
                            .unwrap_or_default();

                        if !strip.eq(add) {
                            let rule = AffixRule {
                                kind: if first.eq("PFX") {
                                    AffixType::Prefix
                                } else {
                                    AffixType::Suffix
                                },
                                flag: flag.to_string(),
                                strip: if strip.eq("0") {
                                    None
                                } else {
                                    Some(strip.to_string())
                                },
                                add: add.to_string(),
                                cond: Regex::new(parts.next().unwrap_or_default()).ok(),
                            };
                            rules.entry(rule.flag.clone()).or_default().push(rule);
                        }
                    }
                    _ => break,
                }
            }
            lines.next();
        }

        Ok(rules)
    }

    fn parse_map(
        lines: &mut Peekable<impl Iterator<Item = Result<String, Error>>>,
    ) -> Result<HashMap<char, char>, DictionaryBuildError> {
        let mut map: HashMap<char, char> = HashMap::new();

        //Stop when cannot read a line
        while let Some(Ok(line)) = lines.peek() {
            if line.is_empty() {
                lines.next();
                continue;
            }
            let mut parts = line.split_whitespace();
            if let Some(first) = parts.next() {
                match first {
                    "MAP" => {
                        if let Some(mut characters) =
                            parts.next().as_ref().as_mut().map(|v| v.chars())
                        {
                            let first = characters.next().unwrap_or('\0');
                            for c in characters {
                                map.insert(c, first);
                            }
                        }
                    }
                    _ => break,
                }
            }
            lines.next();
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // --- helpers ---

    fn make_rule_prefix(strip: Option<String>, add: &str, cond: &str) -> AffixRule {
        AffixRule {
            kind: AffixType::Prefix,
            flag: "X".to_string(),
            strip,
            add: add.to_string(),
            cond: Regex::new(cond).ok(),
        }
    }

    fn make_rule_suffix(strip: Option<String>, add: &str, cond: &str) -> AffixRule {
        AffixRule {
            kind: AffixType::Suffix,
            flag: "X".to_string(),
            strip,
            add: add.to_string(),
            cond: Regex::new(cond).ok(),
        }
    }

    // -----------------------------------
    //  AffixRule::can_apply
    // -----------------------------------
    #[test]
    fn test_can_apply_prefix() {
        let rule = make_rule_prefix(Some("re".to_string()), "pre", r".*"); // match any string
        assert!(rule.can_apply("restart"));
        assert!(!rule.can_apply("start"));
    }

    #[test]
    fn test_can_apply_suffix() {
        let rule = make_rule_suffix(Some("ing".to_string()), "ed", r".*");
        assert!(rule.can_apply("ing"));
        assert!(!rule.can_apply("ingword"));
        assert!(!rule.can_apply("word"));
        assert!(rule.can_apply("walking"));
    }

    // -----------------------------------
    //  AffixRule::apply_rule
    // -----------------------------------
    #[test]
    fn test_apply_rule_prefix() {
        let rule = make_rule_prefix(Some("re".to_string()), "pre", r".*");
        assert_eq!(rule.apply_rule("restart"), "prestart");
    }

    #[test]
    fn test_apply_rule_suffix() {
        let rule = make_rule_suffix(Some("ing".to_string()), "ed", r".*");
        assert_eq!(rule.apply_rule("ing"), "ed");
        assert_eq!(rule.apply_rule("inging"), "inged");
        assert_eq!(rule.apply_rule("walking"), "walked");

        let rule = make_rule_suffix(None, "ed", r".");
        assert_eq!(rule.apply_rule("walk"), "walked");
    }

    // -----------------------------------
    //  parse_affix
    // -----------------------------------
    #[test]
    fn test_parse_affix_basic() {
        let affix_data = "\
PFX A re pre .*
SFX B ing ed .*
";
        let mut lines = Cursor::new(affix_data).lines().peekable();

        let parsed = DictAffix::parse_prefix_suffix(&mut lines).unwrap();

        assert!(parsed.get("A").is_some());
        assert!(parsed.get("B").is_some());

        let rule_a = &parsed.get("A").unwrap()[0];
        assert_eq!(rule_a.kind, AffixType::Prefix);
        assert_eq!(rule_a.strip, Some("re".to_string()));
        assert_eq!(rule_a.add, "pre");

        let rule_b = &parsed.get("B").unwrap()[0];
        assert_eq!(rule_b.kind, AffixType::Suffix);
        assert_eq!(rule_b.strip, Some("ing".to_string()));
        assert_eq!(rule_b.add, "ed");
    }

    // -----------------------------------
    //  DictAffix::apply_rules
    // -----------------------------------
    #[test]
    fn test_apply_rules_prefix() {
        let affix_data = "\
PFX A re pre .*
";

        let dict = DictAffix::try_build(Cursor::new(affix_data)).unwrap();
        dbg!(&dict);
        let results: Vec<String> = dict.apply_rules("restart", "A").collect();
        assert_eq!(results, vec!["prestart"]);
    }

    #[test]
    fn test_apply_rules_suffix() {
        let affix_data = "\
SFX B ing ed .*
";

        let dict = DictAffix::try_build(Cursor::new(affix_data)).unwrap();

        let results: Vec<String> = dict.apply_rules("walking", "B").collect();
        assert_eq!(results, vec!["walked"]); // because "ing" replaced by "ed"
    }

    #[test]
    fn test_apply_rules_suffix_savoir() {
        let affix_data = "\
SFX pE Y 46
SFX pE avoir avoir/n'q'd'l'm't's' savoir
SFX pE avoir achant/n'q'd'l'm't's' savoir
SFX pE avoir u/L'D'Q' savoir
SFX pE avoir us/D'Q' savoir
SFX pE avoir ue/L'D'Q' savoir
SFX pE avoir ues/D'Q' savoir
SFX pE avoir ais/j'n'q'l'm't' savoir
SFX pE avoir ait/n'q'l'm't's' savoir
SFX pE avoir avons/n'q'l't' savoir
SFX pE avoir avez/n'q'l'm' savoir
SFX pE avoir avais/j'n'q'l'm't' savoir
SFX pE avoir avait/n'q'l'm't's' savoir
SFX pE avoir avions/n'q'l't' savoir
SFX pE avoir aviez/n'q'l'm' savoir
SFX pE avoir avaient/n'q'l'm't's' savoir
SFX pE avoir us/j'n'q'l'm't' savoir
SFX pE avoir ut/n'q'l'm't's' savoir
SFX pE avoir ûmes/n'q'l't' savoir
SFX pE avoir ûtes/n'q'l'm' savoir
SFX pE avoir urent/n'q'l'm't's' savoir
SFX pE avoir aurai/j'n'q'l'm't' savoir
SFX pE avoir auras/n'q'l'm't' savoir
SFX pE avoir aura/n'q'l'm't's' savoir
SFX pE avoir aurons/n'q'l't' savoir
SFX pE avoir aurez/n'q'l'm' savoir
SFX pE avoir auront/n'q'l'm't's' savoir
SFX pE avoir aurais/j'n'q'l'm't' savoir
SFX pE avoir aurait/n'q'l'm't's' savoir
SFX pE avoir aurions/n'q'l't' savoir
SFX pE avoir auriez/n'q'l'm' savoir
SFX pE avoir auraient/n'q'l'm't's' savoir
SFX pE avoir ache/j'n'q'l'm't's' savoir
SFX pE avoir aches/n'q'l'm't' savoir
SFX pE avoir achions/n'q'l't' savoir
SFX pE avoir achiez/n'q'l'm' savoir
SFX pE avoir achent/n'q'l'm't's' savoir
SFX pE avoir usse/j'n'q'l'm't' savoir
SFX pE avoir usses/n'q'l'm't' savoir
SFX pE avoir ût/n'q'l'm't's' savoir
SFX pE avoir ussions/n'q'l't' savoir
SFX pE avoir ussiez/n'q'l'm' savoir
SFX pE avoir ussent/n'q'l'm't's' savoir
SFX pE avoir ache/n'l'm't' savoir
SFX pE avoir achons/n'l't' savoir
SFX pE avoir achez/n'l'm' savoir
";

        let dict = DictAffix::try_build(Cursor::new(affix_data)).unwrap();

        let results: Vec<String> = dict.apply_rules("savoir", "pE").collect();
        assert_eq!(
            results,
            vec![
                "sachant",
                "su",
                "sus",
                "sue",
                "sues",
                "sais",
                "sait",
                "savons",
                "savez",
                "savais",
                "savait",
                "savions",
                "saviez",
                "savaient",
                "sus",
                "sut",
                "sûmes",
                "sûtes",
                "surent",
                "saurai",
                "sauras",
                "saura",
                "saurons",
                "saurez",
                "sauront",
                "saurais",
                "saurait",
                "saurions",
                "sauriez",
                "sauraient",
                "sache",
                "saches",
                "sachions",
                "sachiez",
                "sachent",
                "susse",
                "susses",
                "sût",
                "sussions",
                "sussiez",
                "sussent",
                "sache",
                "sachons",
                "sachez"
            ]
        ); // because "ing" replaced by "ed"
    }

    #[test]
    fn test_apply_rules_no_match() {
        let affix_data = "\
PFX A un re .*
";

        let dict = DictAffix::try_build(Cursor::new(affix_data)).unwrap();

        let results: Vec<String> = dict.apply_rules("hello", "A").collect();
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_map_basic() {
        let data = "\
MAP abc
";
        // 'a' becomes the canonical char
        // b → a
        // c → a

        let mut lines = Cursor::new(data).lines().peekable();
        let map = DictAffix::parse_map(&mut lines).unwrap();

        assert_eq!(map.get(&'a'), None); // first char is NOT mapped
        assert_eq!(map.get(&'b'), Some(&'a'));
        assert_eq!(map.get(&'c'), Some(&'a'));
    }

    #[test]
    fn test_parse_map_multiple_lines() {
        let data = "\
MAP abc
MAP xyz
";

        let mut lines = Cursor::new(data).lines().peekable();
        let map = DictAffix::parse_map(&mut lines).unwrap();

        assert_eq!(map.get(&'b'), Some(&'a'));
        assert_eq!(map.get(&'c'), Some(&'a'));
        assert_eq!(map.get(&'y'), Some(&'x'));
        assert_eq!(map.get(&'z'), Some(&'x'));
    }

    #[test]
    fn test_parse_map_unicode() {
        let data = "MAP éèê";

        let mut lines = Cursor::new(data).lines().peekable();
        let map = DictAffix::parse_map(&mut lines).unwrap();

        assert_eq!(map.get(&'è'), Some(&'é'));
        assert_eq!(map.get(&'ê'), Some(&'é'));
    }

    #[test]
    fn test_parse_map_ignores_empty_lines() {
        let data = "\

MAP ab

";

        let mut lines = Cursor::new(data).lines().peekable();
        let map = DictAffix::parse_map(&mut lines).unwrap();

        assert_eq!(map.get(&'b'), Some(&'a'));
    }

    #[test]
    fn test_parse_map_stops_on_non_map_line() {
        let data = "\
MAP ab
PFX A x y .*
MAP cd
";

        // Parser must stop at the first non-MAP ("PFX")
        let mut lines = Cursor::new(data).lines().peekable();
        let map = DictAffix::parse_map(&mut lines).unwrap();

        assert_eq!(map.get(&'b'), Some(&'a'));
        assert_eq!(map.get(&'c'), None); // second MAP must NOT be consumed
        assert_eq!(map.get(&'d'), None);
    }

    #[test]
    fn test_parse_map_malformed_no_chars() {
        let data = "\
MAP
MAP a
";

        let mut lines = Cursor::new(data).lines().peekable();
        let map = DictAffix::parse_map(&mut lines).unwrap();

        // First MAP ignored (no chars), second MAP ignored (only 1 char, no mapping)
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_map_one_char_only() {
        let data = "\
MAP a
";

        let mut lines = Cursor::new(data).lines().peekable();
        let map = DictAffix::parse_map(&mut lines).unwrap();

        assert!(map.is_empty());
    }
}
