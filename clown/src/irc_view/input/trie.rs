#[derive(Default)]
struct TrieNode {
    word_id: Option<usize>,
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
    pub fn new(character: char, word_id: Option<usize>) -> Self {
        Self {
            word_id,
            character,
            nodes: Vec::new(),
        }
    }

    pub fn find_node_index(&self, c: char) -> Option<usize> {
        self.nodes.binary_search_by_key(&c, |v| v.character).ok()
    }

    pub fn insert_node(&mut self, c: char, word_id: Option<usize>) -> &mut TrieNode {
        match self.nodes.binary_search_by_key(&c, |v| v.character) {
            Ok(index) => {
                let n = &mut self.nodes[index];
                if word_id.is_some() {
                    n.word_id = word_id;
                }
                n
            }
            Err(index) => {
                self.nodes.insert(index, TrieNode::new(c, word_id));
                &mut self.nodes[index]
            }
        }
    }
}

pub struct Trie {
    root: TrieNode,
    words: Vec<String>,
}

struct Navigator<'a> {
    start_node: &'a TrieNode,

    result: Option<Vec<usize>>,
}

impl<'a> Navigator<'a> {
    fn new(start_node: &'a TrieNode) -> Self {
        Self {
            start_node,
            result: Some(Vec::new()),
        }
    }

    fn list(&mut self) -> Option<Vec<usize>> {
        self.dfs_list(self.start_node);
        self.result.take()
    }

    fn dfs_list(&mut self, node: &'a TrieNode) {
        if let Some(word_id) = node.word_id
            && let Some(result) = &mut self.result
        {
            result.push(word_id);
        }

        for n in &node.nodes {
            self.dfs_list(n);
        }
    }
}

impl Trie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
            words: Vec::new(),
        }
    }

    pub fn add_word(&mut self, word: &str) {
        let mut current_node = &mut self.root;
        let new_id = self.words.len();
        self.words.push(word.to_string());
        let lower = word.to_lowercase();
        let mut chars = lower.chars().peekable();

        while let Some(next) = chars.next() {
            current_node = current_node.insert_node(next, chars.peek().is_none().then_some(new_id));
        }
    }

    fn navigate_word_mut<F>(&mut self, word: &str, apply: F)
    where
        F: FnOnce(&mut TrieNode),
    {
        let mut current_node = &mut self.root;

        for c in word.chars() {
            if let Some(next_node) = current_node.find_node_index(c) {
                current_node = &mut current_node.nodes[next_node];
            } else {
                break;
            }
        }

        apply(current_node);
    }

    fn navigate_word<F>(&self, word: &str, apply: F)
    where
        F: FnOnce(&TrieNode),
    {
        let mut current_node = &self.root;
        let mut until_end = true;

        for c in word.chars() {
            if let Some(next_node) = current_node.find_node_index(c) {
                current_node = &current_node.nodes[next_node];
            } else {
                until_end = false;
                break;
            }
        }
        if until_end {
            apply(current_node);
        }
    }

    pub fn disable_word(&mut self, word: &str) {
        self.navigate_word_mut(&word.to_lowercase(), |v| v.word_id = None);
    }

    #[cfg(test)]
    pub fn check_word(&self, word: &str) -> bool {
        let mut id = None;
        self.navigate_word(&word.to_lowercase(), |v| id = v.word_id);
        id.is_some_and(|i| self.words[i].eq(word))
    }

    pub fn list(&self, word: &str) -> Option<Vec<String>> {
        let mut result = None;
        self.navigate_word(&word.to_lowercase(), |node| {
            let mut navigator = Navigator::new(node);
            result = navigator.list();
        });

        result.map(|v| {
            v.into_iter()
                .map(|id| self.words[id].to_string())
                .collect::<Vec<String>>()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_single_word() {
        let mut trie = Trie::new();
        trie.add_word("cat");

        assert!(trie.check_word("cat")); // full word
        assert!(!trie.check_word("ca")); // prefix only
        assert!(!trie.check_word("car")); // similar but different
    }

    #[test]
    fn test_list() {
        let mut trie = Trie::new();
        trie.add_word("cat");
        trie.add_word("CaravAne");
        trie.add_word("dog");

        let result = vec!["CaravAne".to_string(), "cat".to_string()];

        assert_eq!(trie.list("c"), Some(result));

        trie.disable_word("caravane");
        let result = vec!["cat".to_string()];

        assert_eq!(trie.list("c"), Some(result));
        assert_eq!(trie.list("A"), None);
    }

    #[test]
    fn test_insert_multiple_words() {
        let mut trie = Trie::new();
        trie.add_word("cat");
        trie.add_word("car");
        trie.add_word("cart");
        trie.add_word("dog");

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
        trie.add_word("a");
        trie.add_word("ab");
        trie.add_word("abc");

        assert!(trie.check_word("a"));
        assert!(trie.check_word("ab"));
        assert!(trie.check_word("abc"));

        assert!(!trie.check_word("abcd"));
        assert!(!trie.check_word("abca"));
    }

    #[test]
    fn test_shared_prefixes_and_end_word_flags() {
        let mut trie = Trie::new();
        trie.add_word("car");
        trie.add_word("card");

        assert!(trie.check_word("car"));
        assert!(trie.check_word("card"));

        assert!(!trie.check_word("carp"));
        assert!(!trie.check_word("ca"));
    }

    #[test]
    fn test_not_existing_words() {
        let mut trie = Trie::new();
        trie.add_word("rust");
        trie.add_word("ruby");

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

        trie.add_word("a");
        assert!(!trie.check_word("")); // still false
    }

    #[test]
    fn test_japanese_words() {
        let mut trie = Trie::new();

        // Add some Japanese words
        trie.add_word("ねこ");
        trie.add_word("いぬ");
        trie.add_word("こんにちは");

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

        trie.add_word("かみ"); // kami
        trie.add_word("かみさま"); // kamisama

        assert!(trie.check_word("かみ"));
        assert!(trie.check_word("かみさま"));

        // prefix only should be false if not end_word
        assert!(!trie.check_word("か"));
        assert!(!trie.check_word("かみさ"));
    }

    #[test]
    fn test_mixed_japanese_and_english() {
        let mut trie = Trie::new();

        trie.add_word("rust");
        trie.add_word("ルスト"); // Rust in katakana

        assert!(trie.check_word("rust"));
        assert!(trie.check_word("ルスト"));

        assert!(!trie.check_word("r"));
        assert!(!trie.check_word("ル"));
    }

    #[test]
    fn test_disable_enable() {
        let mut trie = Trie::new();

        trie.add_word("rust");
        trie.disable_word("rust");

        assert!(!trie.check_word("rust"));
        trie.add_word("rust");
        assert!(trie.check_word("rust"));
    }
}
