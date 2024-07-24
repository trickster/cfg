use core::iter;
use std::rc::Rc;

use cfg_symbol::Symbol;

use cfg_history::{
    BinarizedRhsRange, HistoryGraph, HistoryNode, LinkedHistoryNode, RootHistoryNode,
};

type Id = Symbol;

pub type ExternalOrigin = Option<Id>;
pub type EventId = Option<Id>;
pub type MinimalDistance = Option<Id>;
pub type NullingEliminated = Option<(Symbol, bool)>;
pub type ExternalDottedRule = (u32, u32);
pub type Event = (EventId, MinimalDistance);

#[derive(Copy, Clone)]
enum SymKind {
    Element,
    Separator,
    Other,
}

#[derive(Clone, Default, Debug)]
pub struct History {
    pub dots: Rc<Vec<RuleDot>>,
    pub origin: ExternalOrigin,
    pub nullable: NullingEliminated,
    pub weight: Option<f64>,
    pub sequence: Option<SequenceDetails>,
}

#[derive(Copy, Clone, Debug)]
pub struct RuleDot {
    pub event: Option<(EventId, ExternalDottedRule)>,
    pub distance: MinimalDistance,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct SequenceDetails {
    top: bool,
    rhs: Symbol,
    sep: Option<Symbol>,
}

pub trait HistoryGraphEarleyExt {
    fn final_history(&self) -> Vec<History>;
}

impl HistoryGraphEarleyExt for HistoryGraph {
    fn final_history(&self) -> Vec<History> {
        let mut result: Vec<History> = Vec::with_capacity(self.capacity());
        for node in self.iter() {
            result.push(process_node(node, &result[..]));
        }
        result
    }
}

fn process_node(node: &HistoryNode, prev_histories: &[History]) -> History {
    match node {
        &HistoryNode::Linked {
            prev,
            node: ref linked_node,
        } => {
            let prev_history = prev_histories[prev.get()].clone();
            process_linked(linked_node, prev_history)
        }
        HistoryNode::Root(root) => process_root(*root),
    }
}

fn process_linked(linked_node: &LinkedHistoryNode, mut prev_history: History) -> History {
    match linked_node {
        LinkedHistoryNode::AssignPrecedence { looseness: _, .. } => prev_history,
        &LinkedHistoryNode::Binarize { depth, .. } => prev_history.binarize(depth),
        &LinkedHistoryNode::EliminateNulling {
            which, rhs0, rhs1, ..
        } => prev_history.eliminate_nulling(rhs0, rhs1, which),
        &LinkedHistoryNode::RewriteSequence { top, rhs, sep, .. } => {
            prev_history.sequence = Some(SequenceDetails { top, rhs, sep });
            prev_history
        }
        &LinkedHistoryNode::Weight { weight, .. } => {
            prev_history.weight = Some(weight);
            prev_history
        }
        LinkedHistoryNode::Rhs { rhs, .. } => {
            if let Some(sequence_details) = prev_history.sequence {
                prev_history.rewrite_sequence(sequence_details, &rhs[..]);
            }
            prev_history.dots = (0..=rhs.len())
                .map(|i| RuleDot::new(0, i))
                .collect::<Vec<_>>()
                .into();
            prev_history
        }
        &LinkedHistoryNode::Distances { .. } => prev_history,
    }
}

fn process_root(root_node: RootHistoryNode) -> History {
    match root_node {
        RootHistoryNode::NoOp => History::new(0, 0),
        RootHistoryNode::Rule { lhs: _ } => History::new(0, 0),
        RootHistoryNode::Origin { origin } => History::new(origin as u32, 0),
    }
}

impl RuleDot {
    fn new(id: u32, pos: usize) -> Self {
        RuleDot {
            event: Some((None, (id, pos as u32))),
            distance: None,
        }
    }

    pub fn none() -> Self {
        RuleDot {
            event: None,
            distance: None,
        }
    }

    pub fn trace(self) -> Option<ExternalDottedRule> {
        self.event.map(|x| x.1)
    }

    pub fn event(self) -> Option<(EventId, ExternalDottedRule)> {
        self.event
    }

    pub fn event_without_tracing(self) -> Event {
        (self.event.and_then(|x| x.0), self.distance)
    }

    pub fn distance(&self) -> MinimalDistance {
        self.distance
    }
}

impl History {
    pub fn new(id: u32, len: usize) -> Self {
        History {
            origin: Some(id.into()),
            dots: (0..=len)
                .map(|i| RuleDot::new(id, i))
                .collect::<Vec<_>>()
                .into(),
            ..History::default()
        }
    }

    pub fn origin(&self) -> ExternalOrigin {
        self.origin
    }

    pub fn nullable(&self) -> NullingEliminated {
        self.nullable
    }

    pub fn dot(&self, n: usize) -> RuleDot {
        self.dots[n]
    }

    fn binarize(&self, depth: u32) -> Self {
        let none = RuleDot::none();
        let dots = if self.dots.is_empty() {
            [none; 3]
        } else {
            let dot_len = self.dots.len();
            if depth == 0 {
                if dot_len == 2 {
                    [self.dots[0], none, self.dots[1]]
                } else if dot_len >= 3 {
                    [self.dots[0], self.dots[dot_len - 2], self.dots[dot_len - 1]]
                } else {
                    [self.dots[0], none, none]
                }
            } else {
                [none, self.dots[dot_len - 2 - depth as usize], none]
            }
        };

        let origin = if depth == 0 { self.origin } else { None };

        History {
            origin,
            dots: dots[..].to_vec().into(),
            ..self.clone()
        }
    }

    fn eliminate_nulling(
        &self,
        rhs0: Symbol,
        rhs1: Option<Symbol>,
        subset: BinarizedRhsRange,
    ) -> Self {
        if let BinarizedRhsRange::All = subset {
            History {
                origin: self.origin,
                ..History::default()
            }
        } else {
            let sym = if let BinarizedRhsRange::Right = subset {
                rhs1.unwrap()
            } else {
                rhs0
            };
            History {
                nullable: Some((sym, BinarizedRhsRange::Right == subset)),
                ..self.clone()
            }
        }
    }

    fn rewrite_sequence(&self, details: SequenceDetails, new_rhs: &[Symbol]) -> Self {
        if details.top {
            self.rewrite_sequence_top(details, new_rhs)
        } else {
            self.rewrite_sequence_bottom(details, new_rhs)
        }
    }

    fn rewrite_sequence_top(&self, details: SequenceDetails, new_rhs: &[Symbol]) -> Self {
        let mut bottom = self.rewrite_sequence_bottom(details, new_rhs);
        bottom.origin = self.origin;
        bottom
    }

    fn rewrite_sequence_bottom(&self, details: SequenceDetails, new_rhs: &[Symbol]) -> Self {
        //  -  sym (1) Sep (2)
        //  -  lhs (1) Sep (2) Rhs (1)
        //  -  lhs (0) Rhs (1)
        // (0) Rhs (1)
        // (0) Rhs (1) Sep (2) Rhs (1)
        // (0) Rhs (1) Rhs (1)
        let syms = new_rhs
            .iter()
            .map(|&sym| {
                if sym == details.rhs {
                    SymKind::Element
                } else if Some(sym) == details.sep {
                    SymKind::Separator
                } else {
                    SymKind::Other
                }
            })
            .chain(iter::once(SymKind::Other));
        let mut to_left = SymKind::Other;
        let dots = syms
            .map(|to_right| {
                let dot = match (to_left, to_right) {
                    (_, SymKind::Separator) => self.dots[1],
                    (SymKind::Separator, _) => self.dots[2],
                    (SymKind::Element, _) => self.dots[1],
                    (_, SymKind::Element) => self.dots[0],
                    _ => RuleDot::none(),
                };
                to_left = to_right;
                dot
            })
            .collect::<Vec<_>>()
            .into();
        History {
            dots,
            ..History::default()
        }
    }
}
