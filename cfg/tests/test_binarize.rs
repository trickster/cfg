#![cfg(feature = "cfg-classify")]

use cfg::classify::CfgClassifyUsefulExt;
use cfg::Cfg;
use test_case::test_case;

mod support;

#[test]
fn test_binarize() {
    let mut cfg: Cfg = Cfg::new();
    let [start, a, b, c, x, y] = cfg.sym();

    cfg.rule(start)
        .rhs([a, x, b])
        .rhs([c])
        .rule(b)
        .rhs([a, a])
        .rhs([a, c])
        .rule(c)
        .rhs([x])
        .rhs([y])
        .rule(a)
        .rhs([]);

    cfg.set_roots(&[start]);
    cfg.binarize_and_eliminate_nulling_rules();

    {
        let mut equivalent = Cfg::new();
        let [start, _a, b, c, x, y, g0] = equivalent.sym();
        equivalent
            .rule(start)
            .rhs([g0, b])
            .rhs([c])
            .rule(c)
            .rhs([x])
            .rhs([y])
            .rule(start)
            .rhs([g0])
            .rule(g0)
            .rhs([x])
            .rule(b)
            .rhs([c])
            .rule(a)
            .rhs([])
            .rule(b)
            .rhs([a, a]);
        support::assert_eq_rules(equivalent.rules(), cfg.rules());
    };

    assert!(cfg.usefulness().all_useful());
}

#[test_case(3)]
#[test_case(100)]
#[test_case(1000)]
#[test_case(423)]
fn test_binarize_very_long_rule(num_syms: usize) {
    const RULE_COUNT: usize = 10_000;

    let mut cfg: Cfg = Cfg::new();
    let start = cfg.next_sym();

    let mut long_rhs = cfg
        .sym_source_mut()
        .generate()
        .take(num_syms)
        .collect::<Vec<_>>();
    long_rhs = long_rhs.iter().cloned().cycle().take(RULE_COUNT).collect();
    cfg.rule(start).rhs(long_rhs);

    cfg.set_roots(&[start]);

    assert!(cfg.usefulness().all_useful());
    cfg.limit_rhs_len(Some(2));
    assert_eq!(cfg.rules().count(), RULE_COUNT - 1);

    let mut equivalent = Cfg::new();
    let start = equivalent.next_sym();

    let mut long_rhs = equivalent
        .sym_source_mut()
        .generate()
        .take(100)
        .collect::<Vec<_>>();
    long_rhs = long_rhs.iter().cloned().cycle().take(RULE_COUNT).collect();
    equivalent.rule(start).rhs(long_rhs);
    support::assert_eq_rules(equivalent.rules(), cfg.rules());
}
