//! Analysis of rule usefulness.

use cfg_symbol_bit_matrix::CfgSymbolBitMatrixExt;
use cfg_symbol_bit_matrix::ReachabilityMatrix;

use cfg_grammar::Cfg;
use cfg_grammar::RuleRef;
use cfg_grammar::SymbolBitSet;
use cfg_symbol::Symbol;

/// Contains the information about usefulness of the grammar's rules.
/// Useful rules are both reachable and productive.
pub struct Usefulness {
    reachability: ReachabilityMatrix,
    reachable_syms: SymbolBitSet,
    productivity: SymbolBitSet,
}

/// A reference to a useless rule, together with the reason for its uselessness.
#[derive(Copy, Clone, Debug)]
pub struct UsefulnessForRule<'a> {
    rule: RuleRef<'a>,
    usefulness: RuleUsefulness,
}

#[derive(Copy, Clone, Debug)]
pub struct RuleUsefulness {
    /// Indicates whether the rule is unreachable.
    pub reachable: bool,
    /// Indicates whether the rule is unproductive.
    pub productive: bool,
}

impl<'a> UsefulnessForRule<'a> {
    pub fn rule(&self) -> RuleRef {
        self.rule
    }

    pub fn usefulness(&self) -> RuleUsefulness {
        self.usefulness
    }
}

impl RuleUsefulness {
    fn is_useless(&self) -> bool {
        !self.reachable || !self.productive
    }
}

/// Returns the set of productive symbols.
fn productive_syms(grammar: &mut Cfg) -> SymbolBitSet {
    let mut productive_syms = SymbolBitSet::new();
    productive_syms.terminal(grammar);
    productive_syms.nulling(grammar);
    grammar.rhs_closure(&mut productive_syms);
    productive_syms
}

impl Usefulness {
    /// Analyzes usefulness of the grammar's rules. In particular, it checks for reachable
    /// and productive symbols.
    pub fn new(grammar: &mut Cfg) -> Self {
        let mut productivity = productive_syms(grammar);
        let reachability = grammar.reachability_matrix();
        let mut unused_syms = SymbolBitSet::new();
        unused_syms.used(grammar);
        let mut reachable_syms = SymbolBitSet::from_elem(grammar, false);

        productivity.union(&unused_syms);
        reachable_syms.union(&unused_syms);

        debug_assert_eq!(
            reachability.size(),
            (productivity.len(), productivity.len())
        );

        Usefulness {
            productivity,
            reachability,
            reachable_syms,
        }
    }

    /// Checks whether a symbol is productive. Can be used to determine the precise reason
    /// of a rule's unproductiveness.
    pub fn productivity(&self, sym: Symbol) -> bool {
        self.productivity[sym]
    }

    /// Sets symbol reachability. Takes an array of reachable symbols.
    pub fn reachable(&mut self, syms: impl AsRef<[Symbol]>) {
        for &sym in syms.as_ref() {
            for sym in self.reachability.iter_row_syms(sym) {
                self.reachable_syms.set(sym, true);
            }
        }
    }

    /// Checks whether all rules in the grammar are useful.
    pub fn all_useful(&self) -> bool {
        self.productivity.all() && self.reachable_syms.all()
    }

    /// Checks whether all rules in the grammar are productive.
    pub fn all_productive(&self) -> bool {
        self.productivity.all()
    }

    /// Checks whether all rules in the grammar are reachable.
    pub fn all_reachable(&self) -> bool {
        self.reachable_syms.all()
    }

    pub fn usefulness<'r>(&self, rule: RuleRef<'r>) -> UsefulnessForRule<'r> {
        let productive = rule.rhs.iter().all(|&sym| self.productivity[sym]);
        let reachable = self.reachable_syms[rule.lhs];
        UsefulnessForRule {
            rule,
            usefulness: RuleUsefulness {
                productive,
                reachable,
            },
        }
    }

    fn uselessness_if_useless<'r>(&self, rule: RuleRef<'r>) -> Option<UsefulnessForRule<'r>> {
        let usefulness = self.usefulness(rule);
        if usefulness.usefulness.is_useless() {
            Some(usefulness)
        } else {
            None
        }
    }

    /// Returns an iterator over the grammar's useless rules.
    pub fn useless_rules<'a, 'g>(
        &'a self,
        grammar: &'g Cfg,
    ) -> impl Iterator<Item = UsefulnessForRule<'g>> + 'a
    where
        'g: 'a,
    {
        grammar
            .rules()
            .filter_map(|rule| self.uselessness_if_useless(rule))
    }

    /// Removes useless rules. The language represented by the grammar doesn't change.
    pub fn remove_useless_rules(&self, grammar: &mut Cfg) {
        if !self.all_useful() {
            let productivity = &self.productivity;
            let reachable_syms = &self.reachable_syms;
            let rule_is_useful = |rule: RuleRef| {
                let productive = rule.rhs.iter().all(|&sym| productivity[sym]);
                let reachable = reachable_syms[rule.lhs];
                productive && reachable
            };
            grammar.retain(rule_is_useful);
        }
    }
}

pub trait CfgClassifyUsefulExt {
    fn make_proper(&mut self) -> bool;
    fn usefulness(&mut self) -> Usefulness;
    fn usefulness_with_roots(&mut self, roots: &[Symbol]) -> Usefulness;
}

impl CfgClassifyUsefulExt for Cfg {
    fn usefulness(&mut self) -> Usefulness {
        let mut usefulness = Usefulness::new(self);
        let roots = self.roots();
        usefulness.reachable(roots);
        usefulness
    }

    fn usefulness_with_roots(&mut self, roots: &[Symbol]) -> Usefulness {
        let mut usefulness = Usefulness::new(self);
        usefulness.reachable(roots);
        usefulness
    }

    fn make_proper(&mut self) -> bool {
        let usefulness = self.usefulness();
        let contains_useless_rules = !usefulness.all_useful();
        if contains_useless_rules {
            // for useless in usefulness.useless_rules() {
            //     let rhs: Vec<_> = useless.rule.rhs().iter().map(|t| tok.get(t.usize())).collect();
            //     println!("lhs:{:?} rhs:{:?} unreachable:{} unproductive:{}", tok.get(useless.rule.lhs().usize()), rhs, useless.unreachable, useless.unproductive);
            // }
            // println!("warning: grammar has useless rules");
            usefulness.remove_useless_rules(self);
        }
        !contains_useless_rules
    }
}
