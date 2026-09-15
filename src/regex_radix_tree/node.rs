use std::sync::Arc;

use super::{
    item::Item,
    leaf::Leaf,
    prefix::{Cuts, get_prefix_with_char_size},
};
use crate::regex::LazyRegex;

#[derive(Debug)]
pub struct Node<V> {
    pub(crate) regex: Arc<LazyRegex>,
    pub(crate) children: Vec<Item<V>>,
}

impl<V> Clone for Node<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Node {
            regex: self.regex.clone(),
            children: self.children.clone(),
        }
    }
}

impl<V> Node<V> {
    /// Insert a new item into this node
    pub fn insert(mut self, cuts: &Cuts<'_>, id: String, item: V) -> Item<V> {
        let regex = cuts.regex();
        let mut max_prefix_size = self.regex.original.len() as u32;
        let prefix_size = cuts.shared_with(self.regex.original.as_str());

        if prefix_size < max_prefix_size {
            let prefix = get_prefix_with_char_size(self.regex.original.as_str(), prefix_size);

            let left = Item::Leaf(Leaf::new(regex, id, item, self.regex.options));

            return Item::Node(Node {
                regex: Arc::new(LazyRegex::new_node(prefix, self.regex.options)),
                children: vec![left, Item::Node(self)],
            });
        }

        let mut max_prefix_item = None;

        for i in 0..self.children.len() {
            let prefix_size = cuts.shared_with_over(self.children[i].regex(), max_prefix_size);

            if prefix_size > max_prefix_size {
                max_prefix_size = prefix_size;
                max_prefix_item = Some(i);
            }
        }

        match max_prefix_item {
            Some(child_index) => {
                let mut children = self.children.remove(child_index);
                children = children.insert(cuts, id, item);
                self.children.push(children);
            }
            None => {
                self.children.push(Item::Leaf(Leaf::new(regex, id, item, self.regex.options)));
            }
        }

        Item::Node(self)
    }

    /// Find values associated to this haystack
    pub fn find<'a>(&'a self, haystack: &str, found: &mut Vec<&'a V>) {
        if self.regex.is_match(haystack) {
            for child in &self.children {
                child.find(haystack, found);
            }
        }
    }

    pub fn get(&self, regex: &str) -> Vec<&V> {
        let mut values = Vec::new();

        if regex.starts_with(self.regex.original.as_str()) {
            for child in &self.children {
                values.extend(child.get(regex));
            }
        }

        values
    }

    pub fn get_mut(&mut self, regex: &str) -> Vec<&mut V> {
        let mut values = Vec::new();

        if regex.starts_with(self.regex.original.as_str()) {
            for child in &mut self.children {
                values.extend(child.get_mut(regex));
            }
        }

        values
    }

    pub fn regex(&self) -> &str {
        self.regex.original.as_str()
    }

    /// Remove an item on this tree
    ///
    /// This method returns true if there is no more data so it can be cleaned up
    pub fn remove(mut self, id: &str) -> (Item<V>, Option<V>) {
        let mut removed = None;
        let mut children = Vec::new();

        for child in self.children {
            if removed.is_some() {
                children.push(child);
            } else {
                let (child, value) = child.remove(id);

                if value.is_some() {
                    removed = value;
                }

                if !child.is_empty() {
                    children.push(child);
                }
            }
        }

        if children.len() == 1 {
            return (children.pop().unwrap(), removed);
        }

        self.children = children;

        (Item::Node(self), removed)
    }

    pub fn retain<F>(mut self, f: &F) -> Item<V>
    where
        F: Fn(&str, &mut V) -> bool,
    {
        let mut children = Vec::new();

        for child in self.children {
            let child = child.retain(f);

            if !child.is_empty() {
                children.push(child);
            }
        }

        if children.is_empty() {
            return Item::Empty(self.regex.options);
        }

        if children.len() == 1 {
            return children.pop().unwrap();
        }

        self.children = children;

        Item::Node(self)
    }

    /// Length of node
    pub fn len(&self) -> usize {
        let mut count = 0;

        for child in &self.children {
            count += child.len();
        }

        count
    }

    /// Length of node
    pub fn cached_len(&self) -> usize {
        let mut count = 0;

        if self.regex.compiled.is_some() {
            count += 1;
        }

        for child in &self.children {
            count += child.cached_len();
        }

        count
    }

    pub fn is_empty(&self) -> bool {
        for child in &self.children {
            if !child.is_empty() {
                return false;
            }
        }

        true
    }

    /// Compiles this node's own regex, and nothing under it.
    pub fn compile(&mut self, left: u64) -> u64 {
        if left == 0 || self.regex.compiled.is_some() || self.regex.matches_without_a_regex() {
            return left;
        }

        self.regex = Arc::new(self.regex.compile());

        if self.regex.compiled.is_some() { left - 1 } else { left }
    }

    /// Compiles what a lookup for `haystack` would have had to compile, here and under this
    /// node: the same walk as [`Node::find`], paying as it goes.
    pub fn warm(&mut self, haystack: &str, mut left: u64) -> u64 {
        if left > 0 && self.regex.compiled.is_none() && self.regex.needs_regex_for(haystack) {
            self.regex = Arc::new(self.regex.compile());

            if self.regex.compiled.is_some() {
                left -= 1;
            }
        }

        if !self.regex.is_match(haystack) {
            return left;
        }

        for child in &mut self.children {
            if left == 0 {
                return 0;
            }

            left = child.warm(haystack, left);
        }

        left
    }

    /// Spends a budget of compilations over what is under this node.
    ///
    /// A level at a time, and the whole of a level before anything below it: every lookup that
    /// reaches a branch runs the branch's own regex on the way, so those are worth more than
    /// anything they lead to. What the level leaves over is then split between the branches in
    /// proportion to the number of rules each holds, and every branch spends its share the same
    /// way, level first and the remainder split again.
    ///
    /// The weighting is for where the budget runs out, which it always does somewhere. Before,
    /// it ran out part way through a level, and which branches it had reached by then was
    /// whichever order they happened to have been built in. A branch holding nine rules in ten is
    /// where nine haystacks in ten end up, and is worth nine compilations in ten rather than
    /// whatever is left when its turn comes round.
    pub fn cache(&mut self, mut left: u64) -> u64 {
        // Within the level, whatever the literals cannot rule out goes first. A pattern that
        // requires no text of a haystack is run by every lookup that reaches it; one that
        // requires a long run of it is usually settled without the regex ever being reached.
        // It only tells where the budget runs out, which is somewhere in a level every time.
        let mut level: Vec<usize> = (0..self.children.len()).collect();
        level.sort_unstable_by_key(|index| self.children[*index].prefilter_strength());

        for index in level {
            if left == 0 {
                return 0;
            }

            left = self.children[index].compile(left);
        }

        if left == 0 {
            return 0;
        }

        // Counted once here rather than once per share: `len` walks a whole subtree.
        let sizes: Vec<u64> = self.children.iter().map(|child| child.len() as u64).collect();
        let mut order: Vec<usize> = (0..self.children.len()).collect();
        order.sort_unstable_by_key(|&index| std::cmp::Reverse(sizes[index]));

        let mut rules_left: u64 = sizes.iter().sum();

        for index in order {
            if left == 0 || rules_left == 0 {
                break;
            }

            // Rounded up, so a branch too small to be worth a whole compilation still gets one.
            let share = (left * sizes[index]).div_ceil(rules_left).min(left);
            let unused = self.children[index].cache(share);

            left -= share - unused;
            rules_left -= sizes[index];
        }

        left
    }
}
