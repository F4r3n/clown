use crate::dict::error::DictionaryBuildError;
use regex::Regex;
use std::{collections::HashMap, io::BufRead};

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

pub struct DictAffix {
    rules: HashMap<String, Vec<AffixRule>>,
}

impl DictAffix {
    pub fn try_build<T: BufRead>(reader: T) -> Result<Self, DictionaryBuildError> {
        Ok(Self {
            rules: DictAffix::parse_affix(reader)?,
        })
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
    fn parse_affix<T: BufRead>(
        reader: T,
    ) -> Result<HashMap<String, Vec<AffixRule>>, DictionaryBuildError> {
        let mut rules: HashMap<String, Vec<AffixRule>> = HashMap::new();

        //Stop when cannot read a line
        for line in reader.lines().map_while(Result::ok) {
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
                    _ => {}
                }
            }
        }

        Ok(rules)
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

        let parsed = DictAffix::parse_affix(Cursor::new(affix_data)).unwrap();

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
    fn test_apply_rules_no_match() {
        let affix_data = "\
PFX A un re .*
";

        let dict = DictAffix::try_build(Cursor::new(affix_data)).unwrap();

        let results: Vec<String> = dict.apply_rules("hello", "A").collect();
        assert!(results.is_empty());
    }
}
