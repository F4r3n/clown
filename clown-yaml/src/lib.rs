use std::collections::HashMap;

use yaml_rust2::Yaml;

pub trait ToYaml {
    fn to_yaml(&self) -> Yaml;
}

impl ToYaml for u32 {
    fn to_yaml(&self) -> Yaml {
        Yaml::Integer(*self as i64)
    }
}

impl ToYaml for i32 {
    fn to_yaml(&self) -> Yaml {
        Yaml::Integer(*self as i64)
    }
}

impl ToYaml for &str {
    fn to_yaml(&self) -> Yaml {
        Yaml::String(self.to_string())
    }
}

impl ToYaml for String {
    fn to_yaml(&self) -> Yaml {
        Yaml::String(self.to_string())
    }
}

impl<K, V> ToYaml for HashMap<K, V>
where
    K: Eq + std::hash::Hash + ToYaml,
    V: ToYaml,
{
    fn to_yaml(&self) -> Yaml {
        let mut hash_link = hashlink::LinkedHashMap::new();
        for (key, val) in self {
            hash_link.insert(key.to_yaml(), val.to_yaml());
        }
        Yaml::Hash(hash_link)
    }
}

#[cfg(test)]
mod tests {
    use clown_yaml_derive::ToYaml;
    use yaml_rust2::YamlEmitter;

    use crate::ToYaml;
    #[derive(ToYaml, Default)]
    struct Config {
        test: u32,
    }

    #[test]
    fn simple_serialization() {
        let config = Config::default();
        let mut output = String::new();
        YamlEmitter::new(&mut output)
            .dump(&config.to_yaml())
            .unwrap();
        assert_eq!(output, "---\ntest: 0");
    }
}
