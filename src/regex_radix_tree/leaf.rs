use std::{collections::HashMap, sync::Arc};


use super::{
    item::Item,
    node::Node,
    prefix::{Cuts, get_prefix_with_char_size},
};
use crate::regex::{LazyRegex, RegexOptions};

#[derive(Debug)]
pub struct Leaf<V> {
    pub(crate) values: HashMap<String, V>,
    pub(crate) regex: Arc<LazyRegex>,
}

impl<V> Clone for Leaf<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Leaf {
            values: self.values.clone(),
            regex: self.regex.clone(),
        }
    }
}

impl<V> Leaf<V> {
    pub fn new(regex: &str, id: String, item: V, options: RegexOptions) -> Self {
        let mut values = HashMap::new();
        values.insert(id, item);

        Leaf {
            values,
            regex: Arc::new(LazyRegex::new_leaf(regex, options)),
        }
    }

    /// Insert a new item into this node
    pub fn insert(mut self, cuts: &Cuts<'_>, id: String, item: V) -> Item<V> {
        let regex = cuts.regex();

        if regex == self.regex.original.as_str() {
            self.values.insert(id, item);

            return Item::Leaf(self);
        }

        let prefix = get_prefix_with_char_size(regex, cuts.shared_with(self.regex.original.as_str()));
        let mut leaf_values = HashMap::new();
        leaf_values.insert(id, item);

        let leaf = Item::Leaf(Leaf {
            values: leaf_values,
            regex: Arc::new(LazyRegex::new_leaf(regex, self.regex.options)),
        });

        Item::Node(Node {
            regex: Arc::new(LazyRegex::new_node(prefix, self.regex.options)),
            children: vec![Item::Leaf(self), leaf],
        })
    }

    /// Find values associated to this haystack
    pub fn find<'a>(&'a self, haystack: &str, found: &mut Vec<&'a V>) {
        if self.regex.is_match(haystack) {
            found.extend(self.values.values());
        }
    }

    pub fn get(&self, regex: &str) -> Vec<&V> {
        if self.regex.original.as_str() == regex {
            return self.values.values().collect();
        }

        Vec::new()
    }

    pub fn get_mut(&mut self, regex: &str) -> Vec<&mut V> {
        if self.regex.original.as_str() == regex {
            return self.values.values_mut().collect();
        }

        Vec::new()
    }

    /// Remove an item on this tree
    ///
    /// This method returns true if there is no more data so it can be cleaned up
    pub fn remove(mut self, id: &str) -> (Item<V>, Option<V>) {
        let removed = self.values.remove(id);

        match removed {
            None => (Item::Leaf(self), None),
            Some(value) => {
                if self.values.is_empty() {
                    (Item::Empty(self.regex.options), Some(value))
                } else {
                    (Item::Leaf(self), Some(value))
                }
            }
        }
    }
    pub fn retain<F>(mut self, f: &F) -> Item<V>
    where
        F: Fn(&str, &mut V) -> bool,
    {
        self.values.retain(|k, v| f(k, v));

        if self.values.is_empty() {
            Item::Empty(self.regex.options)
        } else {
            Item::Leaf(self)
        }
    }

    /// Length of node
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn cached_len(&self) -> usize {
        if self.regex.compiled.is_some() {
            return 1;
        }

        0
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn regex(&self) -> &str {
        self.regex.original.as_str()
    }

    /// Compiles this leaf's regex, but only if answering `haystack` would need it.
    pub fn warm(&mut self, haystack: &str, left: u64) -> u64 {
        if left == 0 || self.regex.compiled.is_some() || !self.regex.needs_regex_for(haystack) {
            return left;
        }

        self.regex = Arc::new(self.regex.compile());

        if self.regex.compiled.is_some() { left - 1 } else { left }
    }

    /// Cache current regex according to a limit and a level
    ///
    /// This method must return new limit of element cached (passed limit minus number of element cached)
    /// which allow other node to avoid caching extra node
    ///
    /// Implementation must not cache item if limit is equal to 0
    /// Implementation must not cache item if not caching on the current node level
    ///
    /// Level argument allow to build cache on first level of the tree by priority
    /// Implementation must retain at which level this node is build and not do any caching
    /// if we are not on the current level
    pub fn cache(&mut self, left: u64) -> u64 {
        // Already cached, or never in need of it: a pattern the literals answer on their own
        // never reaches the regex engine, so a compiled copy of it would go unread.
        if self.regex.compiled.is_some() || self.regex.matches_without_a_regex() {
            return left;
        }

        self.regex = Arc::new(self.regex.compile());

        if self.regex.compiled.is_some() {
            return left - 1;
        }

        left
    }
}
