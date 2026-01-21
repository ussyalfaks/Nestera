#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

#[test]
fn test_user_instantiation() {
    let user = User {
        total_balance: 1_000_000,
        savings_count: 3,
    };
    
    assert_eq!(user.total_balance, 1_000_000);
    assert_eq!(user.savings_count, 3);
}

#[test]
fn test_flexi_savings_plan() {
    let plan = SavingsPlan {
        plan_id: 1,
        plan_type: PlanType::Flexi,
        balance: 500_000,
        start_time: 1000000,
        last_deposit: 1000100,
        last_withdraw: 0,
        interest_rate: 500, // 5.00% APY
        is_completed: false,
    };
    
    assert_eq!(plan.plan_id, 1);
    assert_eq!(plan.plan_type, PlanType::Flexi);
    assert_eq!(plan.balance, 500_000);
    assert!(!plan.is_completed);
}

fn test_lock_savings_plan() {
    let locked_until = 2000000;
    let plan = SavingsPlan {
        plan_id: 2,
        plan_type: PlanType::Lock(locked_until),
        balance: 1_000_000,
        start_time: 1000000,
        last_deposit: 1000000,
        last_withdraw: 0,
        interest_rate: 800, // 8.00% APY
        is_completed: false,
    };
    
    assert_eq!(plan.plan_id, 2);
    match plan.plan_type {
        PlanType::Lock(until) => assert_eq!(until, locked_until),
        _ => panic!("Expected Lock plan type"),
    }
}
