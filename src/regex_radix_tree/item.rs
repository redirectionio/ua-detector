use super::{leaf::Leaf, node::Node, prefix::Cuts};
use crate::budget::Budget;
use crate::regex::RegexOptions;
use crate::regex_radix_tree::iter::{ItemIter, ItemIterMut};

#[derive(Debug)]
pub enum Item<V> {
    Empty(RegexOptions),
    Node(Node<V>),
    Leaf(Leaf<V>),
}

impl<V> Clone for Item<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Item::Empty(options) => Item::Empty(*options),
            Item::Node(node) => Item::Node(node.clone()),
            Item::Leaf(leaf) => Item::Leaf(leaf.clone()),
        }
    }
}

impl<V> Item<V> {
    /// Insert a new item into this node
    pub fn insert(self, cuts: &Cuts<'_>, id: String, item: V) -> Item<V> {
        match self {
            Item::Empty(options) => Item::Leaf(Leaf::new(cuts.regex(), id, item, options)),
            Item::Node(node) => node.insert(cuts, id, item),
            Item::Leaf(leaf) => leaf.insert(cuts, id, item),
        }
    }

    /// Find values associated to this haystack
    pub fn find<'a>(&'a self, haystack: &str, found: &mut Vec<&'a V>) {
        match self {
            Item::Empty(_) => (),
            Item::Node(node) => node.find(haystack, found),
            Item::Leaf(leaf) => leaf.find(haystack, found),
        }
    }

    pub fn get(&self, regex: &str) -> Vec<&V> {
        match self {
            Item::Empty(_) => Vec::new(),
            Item::Node(node) => node.get(regex),
            Item::Leaf(leaf) => leaf.get(regex),
        }
    }

    pub fn get_mut(&mut self, regex: &str) -> Vec<&mut V> {
        match self {
            Item::Empty(_) => Vec::new(),
            Item::Node(node) => node.get_mut(regex),
            Item::Leaf(leaf) => leaf.get_mut(regex),
        }
    }

    /// Remove an item on this tree
    ///
    /// This method returns true if there is no more data so it can be cleaned up
    pub fn remove(self, id: &str) -> (Self, Option<V>) {
        match self {
            Item::Empty(_) => (self, None),
            Item::Node(node) => node.remove(id),
            Item::Leaf(leaf) => leaf.remove(id),
        }
    }

    pub fn retain<F>(self, f: &F) -> Item<V>
    where
        F: Fn(&str, &mut V) -> bool,
    {
        match self {
            Item::Empty(_) => self,
            Item::Node(node) => node.retain(f),
            Item::Leaf(leaf) => leaf.retain(f),
        }
    }

    /// Length of node
    pub fn len(&self) -> usize {
        match self {
            Item::Empty(_) => 0,
            Item::Node(node) => node.len(),
            Item::Leaf(leaf) => leaf.len(),
        }
    }

    pub fn cached_len(&self) -> usize {
        match self {
            Item::Empty(_) => 0,
            Item::Node(node) => node.cached_len(),
            Item::Leaf(leaf) => leaf.cached_len(),
        }
    }

    pub fn cached_size(&self) -> u64 {
        match self {
            Item::Empty(_) => 0,
            Item::Node(node) => node.cached_size(),
            Item::Leaf(leaf) => leaf.cached_size(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Item::Empty(_) => true,
            Item::Node(node) => node.is_empty(),
            Item::Leaf(leaf) => leaf.is_empty(),
        }
    }

    /// How much of a haystack this item's own literals rule out. Zero means nothing does, and
    /// its regex is run by every lookup that reaches it.
    pub fn prefilter_strength(&self) -> usize {
        match self {
            Item::Empty(_) => 0,
            Item::Node(node) => node.regex.prefilter_strength(),
            Item::Leaf(leaf) => leaf.regex.prefilter_strength(),
        }
    }

    pub fn regex(&self) -> &str {
        match self {
            Item::Empty(_) => "",
            Item::Node(node) => node.regex(),
            Item::Leaf(leaf) => leaf.regex(),
        }
    }

    pub fn iter(&self) -> ItemIter<'_, V> {
        ItemIter {
            children: std::slice::from_ref(self),
            parent: None,
            values: None,
        }
    }

    pub fn iter_mut(&mut self) -> ItemIterMut<'_, V> {
        ItemIterMut {
            children: std::slice::from_mut(self),
            parent: None,
            values: None,
        }
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
    /// Compiles this item's own regex -- a node's or a leaf's -- and nothing under it.
    pub fn compile(&mut self, left: Budget) -> Budget {
        if left.is_spent() {
            return left;
        }

        match self {
            Item::Empty(_) => left,
            Item::Node(node) => node.compile(left),
            Item::Leaf(leaf) => leaf.cache(left),
        }
    }

    /// Compiles what a lookup for `haystack` would have had to compile.
    pub fn warm(&mut self, haystack: &str, left: Budget) -> Budget {
        if left.is_spent() {
            return left;
        }

        match self {
            Item::Empty(_) => left,
            Item::Node(node) => node.warm(haystack, left),
            Item::Leaf(leaf) => leaf.warm(haystack, left),
        }
    }

    /// Spends a budget over what is under this item. A leaf has nothing under it: its own regex
    /// was compiled by [`Item::compile`], along with the rest of its level.
    pub fn cache(&mut self, left: Budget) -> Budget {
        if left.is_spent() {
            return left;
        }

        match self {
            Item::Empty(_) | Item::Leaf(_) => left,
            Item::Node(node) => node.cache(left),
        }
    }
}