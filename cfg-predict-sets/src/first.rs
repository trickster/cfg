//! FIRST sets.

use std::collections::{BTreeMap, BTreeSet};

use cfg_grammar::symbol::symbol_set::SymbolBitSet;
use cfg_grammar::RuleContainer;
use cfg_symbol::Symbol;

use super::{PerSymbolSets, PredictSets};

/// Collector of FIRST sets.
pub struct FirstSets {
    pub(super) map: PerSymbolSets,
    lookahead: Vec<Option<Symbol>>,
    changed: bool,
    terminal_set: SymbolBitSet,
}

impl FirstSets {
    /// Compute all FIRST sets of the grammar.
    ///
    /// We define a binary relation FIRST(N, S), in which N is related to S
    /// if the grammar has a production of the form `N ⸬= α S β`, where
    /// α is a nullable string of symbols.
    ///
    /// We compute the transitive closure of this relation.
    pub fn new(grammar: &Cfg) -> Self {
        let mut this = FirstSets {
            map: BTreeMap::new(),
            lookahead: vec![],
            changed: true,
            terminal_set: SymbolBitSet::terminal_set(grammar),
        };

        this.collect(grammar);
        this
    }

    /// Calculates a FIRST set for a string of symbols.
    pub fn first_set_for_string(&self, string: &[Symbol]) -> BTreeSet<Option<Symbol>> {
        let mut result = BTreeSet::new();
        for &sym in string {
            let result_cardinality = result.len();
            if self.terminal_set.has_sym(sym) {
                result.insert(Some(sym));
            } else {
                let first_set = self.map.get(&sym).unwrap();
                for &maybe_terminal in first_set {
                    if maybe_terminal.is_some() {
                        result.insert(maybe_terminal);
                    }
                }
            }
            if result_cardinality != result.len() {
                break;
            }
        }
        if result.is_empty() {
            result.insert(None);
        }
        result
    }

    fn collect(&mut self, grammar: &Cfg) {
        while self.changed {
            self.changed = false;
            for rule in grammar.rules() {
                let set_changed = self.rule(rule.lhs, rule.rhs);
                self.changed |= set_changed;
            }
        }
    }

    fn rule(&mut self, lhs: Symbol, rhs: &[Symbol]) -> bool {
        self.first_set_collect(rhs);
        let first_set = self.map.entry(lhs).or_insert_with(BTreeSet::new);
        let prev_cardinality = first_set.len();
        first_set.extend(self.lookahead.iter().cloned());
        self.lookahead.clear();
        prev_cardinality != first_set.len()
    }

    /// Compute a FIRST set.
    fn first_set_collect(&mut self, rhs: &[Symbol]) {
        for &sym in rhs {
            let mut nullable = false;
            if self.terminal_set.has_sym(sym) {
                self.lookahead.push(Some(sym));
            } else {
                match self.map.get(&sym) {
                    None => {
                        // This should only happen during set construction; it
                        // corresponds to an entry that has not yet been
                        // built. Otherwise, it would mean a nonterminal with
                        // no productions. Either way, the resulting first set
                        // should be empty.
                    }
                    Some(set) => {
                        for &maybe_terminal in set {
                            if maybe_terminal.is_some() {
                                self.lookahead.push(maybe_terminal);
                            } else {
                                nullable = true;
                            }
                        }
                    }
                }
            }
            if !nullable {
                // Successfully found a FIRST symbol.
                return;
            }
        }
        self.lookahead.push(None);
    }
}

impl PredictSets for FirstSets {
    /// Returns a reference to FIRST sets.
    fn predict_sets(&self) -> &PerSymbolSets {
        &self.map
    }
}
