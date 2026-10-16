//! Spec 7.1, category matching: the block list covers sub-categories, the accept list drops
//! with `CATEGORY_NOT_ACCEPTED`, the affinity reorders the slot, malformed rules fail the call.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use proxim_conformance::*;
use serde_json::{Value, json};

fn with_category(cat: &str) -> Call {
    let mut c = Call::base();
    c.signposts[0]["category"] = json!(cat);
    c
}

fn drops(c: &Call) -> Vec<String> {
    let s = c.run();
    assert!(s.is_ok(), "{}", s.text);
    s.drops("c-1")
}

#[test]
fn a_blocked_industry_blocks_its_sub_categories_and_nothing_that_merely_shares_letters() {
    for (blocked, cat, dropped) in [
        ("health", "health.hospital", true),
        ("health", "health", true),
        ("health.hospital", "health.hospital", true),
        ("health.hospital", "health", false),
        ("health", "healthy.food", false),
        ("food.cafe", "food.cafe.vegan", true),
        ("food.cafe", "food.cafeteria", false),
    ] {
        let mut c = with_category(cat);
        c.policy["categories_blocked"] = json!([blocked]);
        let want: Vec<String> = if dropped {
            vec!["PARTNER_CATEGORY_BLOCKED".into()]
        } else {
            vec![]
        };
        assert_eq!(drops(&c), want, "block {blocked} vs {cat}");
    }
}

#[test]
fn the_accept_list_admits_listed_industries_only() {
    for (cat, ok) in [
        ("food.cafe", true),
        ("food", true),
        ("retail.grocery", true),
        ("retail", false),
        ("health.hospital", false),
        ("cafe", false),
    ] {
        let mut c = with_category(cat);
        c.policy["categories_accepted"] = json!(["food", "retail.grocery"]);
        let want: Vec<String> = if ok {
            vec![]
        } else {
            vec!["CATEGORY_NOT_ACCEPTED".into()]
        };
        assert_eq!(drops(&c), want, "{cat}");
    }
    // An empty accept list accepts everything (M1 behaviour).
    let mut c = with_category("anything.at_all");
    c.policy["categories_accepted"] = json!([]);
    assert!(drops(&c).is_empty());
}

#[test]
fn the_block_list_is_checked_before_the_accept_list() {
    let mut c = with_category("food.fast_food");
    c.policy["categories_accepted"] = json!(["health"]);
    c.policy["categories_blocked"] = json!(["food"]);
    assert_eq!(drops(&c), vec!["PARTNER_CATEGORY_BLOCKED".to_string()]);
}

#[test]
fn malformed_or_oversized_category_rules_fail_the_whole_call() {
    let bad_rules = [
        json!({"categories_accepted": ["Food"]}),
        json!({"categories_accepted": ["food."]}),
        json!({"categories_accepted": ["a.b.c.d.e"]}),
        json!({"category_affinity": [{"category": "food", "weight_milli": 1}, {"category": "food", "weight_milli": 2}]}),
        json!({"category_affinity": [{"category": "", "weight_milli": 1}]}),
    ];
    for p in bad_rules {
        let mut c = Call::base();
        c.policy = p.clone();
        let s = c.run();
        assert!(!s.is_ok() && s.text.contains("E_PARSE"), "{p}: {}", s.text);
    }
    let many: Vec<String> = (0..257).map(|i| format!("c{i}")).collect();
    let mut c = Call::base();
    c.policy = json!({"categories_accepted": many});
    let s = c.run();
    assert!(!s.is_ok() && s.text.contains("E_LIMIT"), "{}", s.text);
}

/// Two ENTER circles on the same spot; a stationary user inside both; returns who fired.
fn contest(policy: &Value) -> (Vec<String>, Vec<String>) {
    let mut a = enter_circle("a-cafe", 200);
    a["category"] = json!("food.cafe");
    let mut b = enter_circle("b-cinema", 200);
    b["category"] = json!("entertainment.cinema");
    let mut rig = Rig::new().with_signpost(a).with_signpost(b);
    for (k, v) in policy.as_object().unwrap() {
        rig = rig.with_policy(k, v.clone());
    }
    for i in 0..120 {
        let s = rig.step(&fix_at(BASE_T + i * 1000, ORIGIN));
        assert!(s.is_ok(), "{}", s.text);
        let fired: Vec<String> = ["a-cafe", "b-cinema"]
            .into_iter()
            .filter(|id| s.fired(id))
            .map(String::from)
            .collect();
        if !fired.is_empty() {
            let lost: Vec<String> = ["a-cafe", "b-cinema"]
                .into_iter()
                .filter(|id| s.drops(id).contains(&"LOST_SLOT".to_string()))
                .map(String::from)
                .collect();
            return (fired, lost);
        }
    }
    panic!("nothing fired");
}

#[test]
fn affinity_decides_the_slot_between_simultaneous_adverts() {
    // Equal score: the id order decides (spec 7), so the cafe wins.
    let (fired, lost) = contest(&json!({}));
    assert_eq!(
        (fired, lost),
        (vec!["a-cafe".into()], vec!["b-cinema".into()])
    );
    // The partner prefers entertainment: the cinema wins the same contest.
    let (fired, lost) = contest(&json!({"category_affinity": [
        {"category": "entertainment", "weight_milli": 2000}
    ]}));
    assert_eq!(
        (fired, lost),
        (vec!["b-cinema".into()], vec!["a-cafe".into()])
    );
    // The longest matching entry applies: food 2500 loses to food.cafe 500 for the cafe.
    let (fired, _) = contest(&json!({"category_affinity": [
        {"category": "food", "weight_milli": 2500},
        {"category": "food.cafe", "weight_milli": 500}
    ]}));
    assert_eq!(fired, vec!["b-cinema".to_string()]);
    // A weight above 3000 is clamped to 3000, so it cannot beat an explicit 3000 on a
    // lower id.
    let (fired, _) = contest(&json!({"category_affinity": [
        {"category": "food.cafe", "weight_milli": 3000},
        {"category": "entertainment", "weight_milli": 60000}
    ]}));
    assert_eq!(fired, vec!["a-cafe".to_string()]);
}
