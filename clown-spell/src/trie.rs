#[derive(Default)]
struct TrieNode {
    end_word: bool,
    character: char,
    nodes: Vec<TrieNode>,
}

impl PartialEq for TrieNode {
    fn eq(&self, other: &Self) -> bool {
        self.character == other.character
    }
}

impl Eq for TrieNode {}

impl PartialOrd for TrieNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TrieNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.character.cmp(&other.character)
    }
}

impl TrieNode {
    pub fn new(character: char, end_word: bool) -> Self {
        Self {
            end_word,
            character,
            nodes: Vec::new(),
        }
    }

    pub fn find_node(&self, c: char) -> Option<&TrieNode> {
        if let Ok(index) = self.nodes.binary_search_by_key(&c, |v| v.character) {
            Some(&self.nodes[index])
        } else {
            None
        }
    }

    pub fn insert_node(&mut self, c: char, end: bool) -> &mut TrieNode {
        match self.nodes.binary_search_by_key(&c, |v| v.character) {
            Ok(index) => {
                let n = &mut self.nodes[index];
                n.end_word |= end;
                n
            }
            Err(index) => {
                self.nodes.insert(index, TrieNode::new(c, end));
                &mut self.nodes[index]
            }
        }
    }
}

pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
        }
    }

    pub fn add_word(&mut self, word: String) {
        let mut current_node = &mut self.root;
        let mut chars = word.chars().peekable();
        while let Some(next) = chars.next() {
            current_node = current_node.insert_node(next, chars.peek().is_none());
        }
    }

    pub fn check_word(&self, word: &str) -> bool {
        let mut current_node = &self.root;
        for c in word.chars() {
            if let Some(next_node) = current_node.find_node(c) {
                current_node = next_node;
            } else {
                return false;
            }
        }
        current_node.end_word
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_single_word() {
        let mut trie = Trie::new();
        trie.add_word("cat".to_string());

        assert!(trie.check_word("cat")); // full word
        assert!(!trie.check_word("ca")); // prefix only
        assert!(!trie.check_word("car")); // similar but different
    }

    #[test]
    fn test_insert_multiple_words() {
        let mut trie = Trie::new();
        trie.add_word("cat".to_string());
        trie.add_word("car".to_string());
        trie.add_word("cart".to_string());
        trie.add_word("dog".to_string());

        assert!(trie.check_word("cat"));
        assert!(trie.check_word("car"));
        assert!(trie.check_word("cart"));
        assert!(trie.check_word("dog"));

        assert!(!trie.check_word("ca")); // prefix, not word
        assert!(!trie.check_word("do")); // prefix
        assert!(!trie.check_word("dogs")); // extension
        assert!(!trie.check_word("c")); // prefix
    }

    #[test]
    fn test_prefix_chain() {
        let mut trie = Trie::new();
        trie.add_word("a".to_string());
        trie.add_word("ab".to_string());
        trie.add_word("abc".to_string());

        assert!(trie.check_word("a"));
        assert!(trie.check_word("ab"));
        assert!(trie.check_word("abc"));

        assert!(!trie.check_word("abcd"));
        assert!(!trie.check_word("abca"));
    }

    #[test]
    fn test_shared_prefixes_and_end_word_flags() {
        let mut trie = Trie::new();
        trie.add_word("car".to_string());
        trie.add_word("card".to_string());

        assert!(trie.check_word("car"));
        assert!(trie.check_word("card"));

        assert!(!trie.check_word("carp"));
        assert!(!trie.check_word("ca"));
    }

    #[test]
    fn test_not_existing_words() {
        let mut trie = Trie::new();
        trie.add_word("rust".to_string());
        trie.add_word("ruby".to_string());

        assert!(trie.check_word("rust"));
        assert!(trie.check_word("ruby"));

        assert!(!trie.check_word("ru"));
        assert!(!trie.check_word("rusty"));
        assert!(!trie.check_word("python"));
    }

    #[test]
    fn test_empty_word() {
        let mut trie = Trie::new();

        assert!(!trie.check_word("")); // empty string is never a word

        trie.add_word("a".to_string());
        assert!(!trie.check_word("")); // still false
    }

    #[test]
    fn test_japanese_words() {
        let mut trie = Trie::new();

        // Add some Japanese words
        trie.add_word("ねこ".to_string());
        trie.add_word("いぬ".to_string());
        trie.add_word("こんにちは".to_string());

        // Full words should return true
        assert!(trie.check_word("ねこ"));
        assert!(trie.check_word("いぬ"));
        assert!(trie.check_word("こんにちは"));

        // Prefixes should return false
        assert!(!trie.check_word("ね"));
        assert!(!trie.check_word("こん"));
        assert!(!trie.check_word("にち"));

        // Non-existing words should return false
        assert!(!trie.check_word("さる")); // saru = monkey
        assert!(!trie.check_word("こんばんは")); // konbanwa = good evening
    }

    #[test]
    fn test_japanese_shared_prefixes() {
        let mut trie = Trie::new();

        trie.add_word("かみ".to_string()); // kami
        trie.add_word("かみさま".to_string()); // kamisama

        assert!(trie.check_word("かみ"));
        assert!(trie.check_word("かみさま"));

        // prefix only should be false if not end_word
        assert!(!trie.check_word("か"));
        assert!(!trie.check_word("かみさ"));
    }

    #[test]
    fn test_mixed_japanese_and_english() {
        let mut trie = Trie::new();

        trie.add_word("rust".to_string());
        trie.add_word("ルスト".to_string()); // Rust in katakana

        assert!(trie.check_word("rust"));
        assert!(trie.check_word("ルスト"));

        assert!(!trie.check_word("r"));
        assert!(!trie.check_word("ル"));
    }
}
