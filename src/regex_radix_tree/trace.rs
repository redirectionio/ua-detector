//! What one walk of the tree did, node by node, and what each step of it cost.
//!
//! A trace takes the same route a [`find`](super::RegexTreeMap::find) would: it runs the same
//! regexes, in the same order, and descends only where one matched. What it adds is a clock
//! around every one of them, which is enough to say where a lookup spends itself -- and, since
//! a regex outside the cache budget is compiled again at every lookup that reaches it, to tell
//! a branch that is slow from one that is merely being compiled over and over.

use std::time::Duration;

use super::{item::Item, leaf::Leaf, node::Node};
use crate::regex::{MatchPath, Measure};

#[derive(Debug, Clone)]
pub struct Trace<'a, V> {
    pub regex: String,
    pub count: u64,
    pub matched: bool,
    pub children: Vec<Trace<'a, V>>,
    pub values: Vec<&'a V>,
    /// What this node's own regex answered, and what it cost. Nothing under it is counted here.
    pub measure: Measure,
}

impl<'a, V> Trace<'a, V> {
    /// Visits the whole subtree, root first, handing each node its depth.
    pub fn walk(&self, visit: &mut impl FnMut(&Trace<'a, V>, usize)) {
        self.walk_from(0, visit);
    }

    fn walk_from(&self, depth: usize, visit: &mut impl FnMut(&Trace<'a, V>, usize)) {
        visit(self, depth);

        for child in &self.children {
            child.walk_from(depth + 1, visit);
        }
    }

    /// What the whole subtree cost, this node's own regex included.
    pub fn elapsed(&self) -> Duration {
        self.children
            .iter()
            .fold(self.measure.elapsed, |total, child| total + child.elapsed())
    }

    /// What of that went on compiling regexes the cache budget did not cover.
    pub fn compile(&self) -> Duration {
        self.children
            .iter()
            .fold(self.measure.compile, |total, child| total + child.compile())
    }

    /// How many regexes the walk ran, over the whole subtree.
    pub fn visited(&self) -> usize {
        self.children.iter().map(Trace::visited).sum::<usize>() + 1
    }

    /// How many of them took a given route.
    pub fn taking(&self, path: MatchPath) -> usize {
        let under: usize = self.children.iter().map(|child| child.taking(path)).sum();

        under + usize::from(self.measure.path == path)
    }

    /// Whether this node is the end of the walk rather than a branch of it.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// The same trace with the values dropped, which is what lets a caller hand one out when
    /// the type behind them is its own business.
    pub fn erased(&self) -> Trace<'static, ()> {
        Trace {
            regex: self.regex.clone(),
            count: self.count,
            matched: self.matched,
            children: self.children.iter().map(Trace::erased).collect(),
            values: Vec::new(),
            measure: self.measure,
        }
    }
}

impl<V> Leaf<V> {
    pub fn trace(&self, haystack: &str) -> Trace<'_, V> {
        let measure = self.regex.measure_match(haystack);

        Trace {
            regex: self.regex.original.clone(),
            matched: measure.matched,
            count: self.values.len() as u64,
            children: Vec::new(),
            values: self.values.values().collect(),
            measure,
        }
    }
}
impl<V> Node<V> {
    pub fn trace(&self, haystack: &str) -> Trace<'_, V> {
        let mut children = Vec::new();
        let measure = self.regex.measure_match(haystack);

        if measure.matched {
            for child in &self.children {
                children.push(child.trace(haystack));
            }
        }

        Trace {
            regex: self.regex.original.clone(),
            matched: measure.matched,
            count: self.len() as u64,
            children,
            values: Vec::new(),
            measure,
        }
    }
}

impl<V> Item<V> {
    pub fn trace(&self, haystack: &str) -> Trace<'_, V> {
        match self {
            Item::Empty(_) => Trace {
                regex: "".to_string(),
                matched: true,
                count: 0,
                children: Vec::new(),
                values: Vec::new(),
                measure: Measure {
                    matched: true,
                    path: MatchPath::Empty,
                    elapsed: Duration::ZERO,
                    compile: Duration::ZERO,
                },
            },
            Item::Node(node) => node.trace(haystack),
            Item::Leaf(leaf) => leaf.trace(haystack),
        }
    }
}
