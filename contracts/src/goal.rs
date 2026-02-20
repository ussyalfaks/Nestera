use soroban_sdk::{symbol_short, Address, Env, Vec};

use crate::calculate_fee;
use crate::ensure_not_paused;
use crate::errors::SavingsError;
use crate::storage_types::{DataKey, GoalSave, User};
use crate::ttl;
use crate::users;

pub fn create_goal_save(
    env: &Env,
    user: Address,
    goal_name: soroban_sdk::Symbol,
    target_amount: i128,
    initial_deposit: i128,
) -> Result<u64, SavingsError> {
    ensure_not_paused(env)?;
    user.require_auth();

    if target_amount <= 0 {
        return Err(SavingsError::InvalidAmount);
    }

    if initial_deposit < 0 {
        return Err(SavingsError::InvalidAmount);
    }

    if !users::user_exists(env, &user) {
        return Err(SavingsError::UserNotFound);
    }

    // Calculate protocol fee on initial deposit
    let fee_bps: u32 = env
        .storage()
        .instance()
        .get(&DataKey::PlatformFee)
        .unwrap_or(0);

    let fee_amount = calculate_fee(initial_deposit, fee_bps);
    let net_initial_deposit = initial_deposit
        .checked_sub(fee_amount)
        .ok_or(SavingsError::Underflow)?;

    let current_time = env.ledger().timestamp();
    let goal_id = get_next_goal_id(env);

    let goal_save = GoalSave {
        id: goal_id,
        owner: user.clone(),
        goal_name: goal_name.clone(),
        target_amount,
        current_amount: net_initial_deposit,
        interest_rate: 500,
        start_time: current_time,
        is_completed: net_initial_deposit >= target_amount,
        is_withdrawn: false,
    };

    env.storage()
        .persistent()
        .set(&DataKey::GoalSave(goal_id), &goal_save);

    // Transfer fee to treasury if fee > 0
    if fee_amount > 0 {
        if let Some(fee_recipient) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::FeeRecipient)
        {
            let fee_key = DataKey::TotalBalance(fee_recipient.clone());
            let current_fee_balance = env
                .storage()
                .persistent()
                .get::<DataKey, i128>(&fee_key)
                .unwrap_or(0i128);
            let new_fee_balance = current_fee_balance
                .checked_add(fee_amount)
                .ok_or(SavingsError::Overflow)?;
            env.storage().persistent().set(&fee_key, &new_fee_balance);
            env.events().publish(
                (symbol_short!("gdep_fee"), fee_recipient, goal_id),
                fee_amount,
            );
        }
    }

    add_goal_to_user(env, &user, goal_id);
    increment_next_goal_id(env);

    // Extend TTL for new goal save and user data
    ttl::extend_goal_ttl(env, goal_id);
    ttl::extend_user_plan_list_ttl(env, &DataKey::UserGoalSaves(user.clone()));

    Ok(goal_id)
}

pub fn deposit_to_goal_save(
    env: &Env,
    user: Address,
    goal_id: u64,
    amount: i128,
) -> Result<(), SavingsError> {
    ensure_not_paused(env)?;
    user.require_auth();

    if amount <= 0 {
        return Err(SavingsError::InvalidAmount);
    }

    let mut goal_save = get_goal_save(env, goal_id).ok_or(SavingsError::PlanNotFound)?;

    if goal_save.owner != user {
        return Err(SavingsError::Unauthorized);
    }

    if goal_save.is_completed {
        return Err(SavingsError::PlanCompleted);
    }

    // Calculate protocol fee
    let fee_bps: u32 = env
        .storage()
        .instance()
        .get(&DataKey::PlatformFee)
        .unwrap_or(0);

    let fee_amount = calculate_fee(amount, fee_bps);
    let net_amount = amount
        .checked_sub(fee_amount)
        .ok_or(SavingsError::Underflow)?;

    goal_save.current_amount = goal_save
        .current_amount
        .checked_add(net_amount)
        .ok_or(SavingsError::Overflow)?;

    if goal_save.current_amount >= goal_save.target_amount {
        goal_save.is_completed = true;
    }

    env.storage()
        .persistent()
        .set(&DataKey::GoalSave(goal_id), &goal_save);

    // Extend TTL on deposit
    ttl::extend_goal_ttl(env, goal_id);
    ttl::extend_user_ttl(env, &user);

    // Transfer fee to treasury if fee > 0
    if fee_amount > 0 {
        if let Some(fee_recipient) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::FeeRecipient)
        {
            let fee_key = DataKey::TotalBalance(fee_recipient.clone());
            let current_fee_balance = env
                .storage()
                .persistent()
                .get::<DataKey, i128>(&fee_key)
                .unwrap_or(0i128);
            let new_fee_balance = current_fee_balance
                .checked_add(fee_amount)
                .ok_or(SavingsError::Overflow)?;
            env.storage().persistent().set(&fee_key, &new_fee_balance);
            env.events().publish(
                (symbol_short!("gdep_fee"), fee_recipient, goal_id),
                fee_amount,
            );
        }
    }

    Ok(())
}

pub fn withdraw_completed_goal_save(
    env: &Env,
    user: Address,
    goal_id: u64,
) -> Result<i128, SavingsError> {
    ensure_not_paused(env)?;
    user.require_auth();

    if !users::user_exists(env, &user) {
        return Err(SavingsError::UserNotFound);
    }

    let mut goal_save = get_goal_save(env, goal_id).ok_or(SavingsError::PlanNotFound)?;

    if goal_save.owner != user {
        return Err(SavingsError::Unauthorized);
    }

    if !goal_save.is_completed {
        return Err(SavingsError::TooEarly);
    }

    if goal_save.is_withdrawn {
        return Err(SavingsError::PlanCompleted);
    }

    // Calculate protocol fee on withdrawal
    let fee_bps: u32 = env
        .storage()
        .instance()
        .get(&DataKey::PlatformFee)
        .unwrap_or(0);

    let fee_amount = calculate_fee(goal_save.current_amount, fee_bps);
    let net_amount = goal_save
        .current_amount
        .checked_sub(fee_amount)
        .ok_or(SavingsError::Underflow)?;

    goal_save.is_withdrawn = true;

    env.storage()
        .persistent()
        .set(&DataKey::GoalSave(goal_id), &goal_save);

    let user_key = DataKey::User(user.clone());
    if let Some(mut user_data) = env.storage().persistent().get::<DataKey, User>(&user_key) {
        user_data.total_balance = user_data
            .total_balance
            .checked_add(net_amount)
            .ok_or(SavingsError::Overflow)?;
        env.storage().persistent().set(&user_key, &user_data);
    }

    // Extend TTL (withdrawn goals get shorter extension)
    ttl::extend_goal_ttl(env, goal_id);
    ttl::extend_user_ttl(env, &user);

    // Transfer fee to treasury if fee > 0
    if fee_amount > 0 {
        if let Some(fee_recipient) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::FeeRecipient)
        {
            let fee_key = DataKey::TotalBalance(fee_recipient.clone());
            let current_fee_balance = env
                .storage()
                .persistent()
                .get::<DataKey, i128>(&fee_key)
                .unwrap_or(0i128);
            let new_fee_balance = current_fee_balance
                .checked_add(fee_amount)
                .ok_or(SavingsError::Overflow)?;
            env.storage().persistent().set(&fee_key, &new_fee_balance);
            env.events().publish(
                (symbol_short!("gwth_fee"), fee_recipient, goal_id),
                fee_amount,
            );
        }
    }

    Ok(net_amount)
}

pub fn break_goal_save(env: &Env, user: Address, goal_id: u64) -> Result<i128, SavingsError> {
    ensure_not_paused(env)?;
    user.require_auth();

    if !users::user_exists(env, &user) {
        return Err(SavingsError::UserNotFound);
    }

    let mut goal_save = get_goal_save(env, goal_id).ok_or(SavingsError::PlanNotFound)?;

    if goal_save.owner != user {
        return Err(SavingsError::Unauthorized);
    }

    if goal_save.is_completed {
        return Err(SavingsError::PlanCompleted);
    }

    if goal_save.is_withdrawn {
        return Err(SavingsError::PlanCompleted);
    }

    let fee_bps: u32 = env
        .storage()
        .instance()
        .get(&DataKey::EarlyBreakFeeBps)
        .unwrap_or(0);

    if fee_bps > 10_000 {
        return Err(SavingsError::InvalidAmount);
    }

    let fee_amount = if fee_bps == 0 {
        0
    } else {
        goal_save
            .current_amount
            .checked_mul(fee_bps as i128)
            .ok_or(SavingsError::Overflow)?
            / 10_000
    };

    let net_amount = goal_save
        .current_amount
        .checked_sub(fee_amount)
        .ok_or(SavingsError::Underflow)?;

    goal_save.is_withdrawn = true;

    env.storage()
        .persistent()
        .set(&DataKey::GoalSave(goal_id), &goal_save);

    let user_key = DataKey::User(user.clone());
    if let Some(mut user_data) = env.storage().persistent().get::<DataKey, User>(&user_key) {
        user_data.total_balance = user_data
            .total_balance
            .checked_add(net_amount)
            .ok_or(SavingsError::Overflow)?;
        env.storage().persistent().set(&user_key, &user_data);
    }

    if fee_amount > 0 {
        if let Some(fee_recipient) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::FeeRecipient)
        {
            let fee_key = DataKey::TotalBalance(fee_recipient.clone());
            let current_fee_balance = env
                .storage()
                .persistent()
                .get::<DataKey, i128>(&fee_key)
                .unwrap_or(0i128);
            let new_fee_balance = current_fee_balance
                .checked_add(fee_amount)
                .ok_or(SavingsError::Overflow)?;
            env.storage().persistent().set(&fee_key, &new_fee_balance);

            // Extend TTL on fee storage
            ttl::extend_config_ttl(env, &fee_key);

            env.events().publish(
                (symbol_short!("brk_fee"), fee_recipient, goal_id),
                fee_amount,
            );
        }
    }

    env.events().publish(
        (symbol_short!("goal_brk"), user.clone(), goal_id),
        net_amount,
    );

    remove_goal_from_user(env, &user, goal_id);

    // Extend TTL (withdrawn goals get shorter extension)
    ttl::extend_goal_ttl(env, goal_id);
    ttl::extend_user_ttl(env, &user);

    Ok(net_amount)
}

pub fn get_goal_save(env: &Env, goal_id: u64) -> Option<GoalSave> {
    let goal_save = env.storage().persistent().get(&DataKey::GoalSave(goal_id));
    if goal_save.is_some() {
        // Extend TTL on read
        ttl::extend_goal_ttl(env, goal_id);
    }
    goal_save
}

pub fn get_user_goal_saves(env: &Env, user: &Address) -> Vec<u64> {
    let list_key = DataKey::UserGoalSaves(user.clone());
    let goals = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));

    // Extend TTL on list access
    if goals.len() > 0 {
        ttl::extend_user_plan_list_ttl(env, &list_key);
    }

    goals
}

fn get_next_goal_id(env: &Env) -> u64 {
    let counter_key = DataKey::NextGoalId;
    let id = env.storage().persistent().get(&counter_key).unwrap_or(1u64);

    // Extend TTL on counter access
    ttl::extend_counter_ttl(env, &counter_key);

    id
}

fn increment_next_goal_id(env: &Env) {
    let current_id = get_next_goal_id(env);
    let counter_key = DataKey::NextGoalId;
    env.storage()
        .persistent()
        .set(&counter_key, &(current_id + 1));

    // Extend TTL on counter update
    ttl::extend_counter_ttl(env, &counter_key);
}

fn add_goal_to_user(env: &Env, user: &Address, goal_id: u64) {
    let mut user_goals = get_user_goal_saves(env, user);
    user_goals.push_back(goal_id);
    env.storage()
        .persistent()
        .set(&DataKey::UserGoalSaves(user.clone()), &user_goals);
}

fn remove_goal_from_user(env: &Env, user: &Address, goal_id: u64) {
    let user_goals = get_user_goal_saves(env, user);
    let mut new_goals = Vec::new(env);

    for i in 0..user_goals.len() {
        if let Some(id) = user_goals.get(i) {
            if id != goal_id {
                new_goals.push_back(id);
            }
        }
    }

    env.storage()
        .persistent()
        .set(&DataKey::UserGoalSaves(user.clone()), &new_goals);
}

#[cfg(test)]
mod tests {
    use crate::{NesteraContract, NesteraContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

    fn setup_test_env() -> (Env, NesteraContractClient<'static>) {
        let env = Env::default();
        let contract_id = env.register(NesteraContract, ());
        let client = NesteraContractClient::new(&env, &contract_id);
        (env, client)
    }

    fn setup_admin_env() -> (Env, NesteraContractClient<'static>, Address) {
        let env = Env::default();
        let contract_id = env.register(NesteraContract, ());
        let client = NesteraContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let admin_pk = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);

        env.mock_all_auths();
        client.initialize(&admin, &admin_pk);

        (env, client, admin)
    }

    #[test]
    fn test_create_goal_save_success() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "vacation");
        let target = 10000i128;
        let initial = 1000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        assert_eq!(goal_id, 1);

        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.owner, user);
        assert_eq!(goal_save.target_amount, target);
        assert_eq!(goal_save.current_amount, initial);
        assert!(!goal_save.is_completed);
        assert!(!goal_save.is_withdrawn);
    }

    #[test]
    fn test_deposit_to_goal_save() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "house");
        let target = 5000i128;
        let initial = 1000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        client.deposit_to_goal_save(&user, &goal_id, &2000);

        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.current_amount, 3000);
        assert!(!goal_save.is_completed);
    }

    #[test]
    fn test_goal_completion_on_deposit() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "laptop");
        let target = 5000i128;
        let initial = 3000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        client.deposit_to_goal_save(&user, &goal_id, &2000);

        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.current_amount, 5000);
        assert!(goal_save.is_completed);
    }

    #[test]
    fn test_withdraw_completed_goal_save_success() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "car");
        let target = 1000i128;
        let initial = 1000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);

        let goal_save = client.get_goal_save_detail(&goal_id);
        assert!(goal_save.is_completed);

        let amount = client.withdraw_completed_goal_save(&user, &goal_id);
        assert_eq!(amount, 1000);

        let goal_save_after = client.get_goal_save_detail(&goal_id);
        assert!(goal_save_after.is_withdrawn);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #51)")]
    fn test_withdraw_incomplete_goal_fails() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "bike");
        let target = 5000i128;
        let initial = 1000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);

        client.withdraw_completed_goal_save(&user, &goal_id);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #23)")]
    fn test_withdraw_already_withdrawn_fails() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "fund");
        let target = 1000i128;
        let initial = 1000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        client.withdraw_completed_goal_save(&user, &goal_id);
        client.withdraw_completed_goal_save(&user, &goal_id);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_withdraw_unauthorized_fails() {
        let (env, client) = setup_test_env();
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user1);
        client.initialize_user(&user2);

        let goal_name = Symbol::new(&env, "test");
        let target = 1000i128;
        let initial = 1000i128;

        let goal_id = client.create_goal_save(&user1, &goal_name, &target, &initial);
        client.withdraw_completed_goal_save(&user2, &goal_id);
    }

    #[test]
    fn test_break_goal_save_success() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "emergency");
        let target = 5000i128;
        let initial = 2000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        let net_amount = client.break_goal_save(&user, &goal_id);
        assert_eq!(net_amount, initial);

        let goal_save = client.get_goal_save_detail(&goal_id);
        assert!(goal_save.is_withdrawn);

        let user_goals = client.get_user_goal_saves(&user);
        assert_eq!(user_goals.len(), 0);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #23)")]
    fn test_break_completed_goal_fails() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "done");
        let target = 1000i128;
        let initial = 1000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        client.break_goal_save(&user, &goal_id);
    }

    #[test]
    fn test_break_goal_save_applies_fee_and_routes() {
        let (env, client, _admin) = setup_admin_env();
        let user = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);
        assert!(client.try_set_fee_recipient(&treasury).is_ok());
        assert!(client.try_set_early_break_fee_bps(&500).is_ok()); // 5%

        let goal_name = Symbol::new(&env, "emergency");
        let target = 10_000i128;
        let initial = 2_000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        let net_amount = client.break_goal_save(&user, &goal_id);

        assert_eq!(net_amount, 1_900);
        assert_eq!(client.get_protocol_fee_balance(&treasury), 100);
    }

    #[test]
    fn test_break_goal_save_fee_rounds_down() {
        let (env, client, _admin) = setup_admin_env();
        let user = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);
        assert!(client.try_set_fee_recipient(&treasury).is_ok());
        assert!(client.try_set_early_break_fee_bps(&125).is_ok()); // 1.25%

        let goal_name = Symbol::new(&env, "rounding");
        let target = 10_000i128;
        let initial = 3_333i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        let net_amount = client.break_goal_save(&user, &goal_id);

        // fee = floor(3333 * 125 / 10000) = 41
        assert_eq!(net_amount, 3_292);
        assert_eq!(client.get_protocol_fee_balance(&treasury), 41);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_break_unauthorized_fails() {
        let (env, client) = setup_test_env();
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user1);
        client.initialize_user(&user2);

        let goal_name = Symbol::new(&env, "other");
        let target = 5000i128;
        let initial = 2000i128;

        let goal_id = client.create_goal_save(&user1, &goal_name, &target, &initial);
        client.break_goal_save(&user2, &goal_id);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #41)")]
    fn test_create_goal_save_invalid_target_amount() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "invalid");
        let target = 0i128;
        let initial = 100i128;

        client.create_goal_save(&user, &goal_name, &target, &initial);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_create_goal_save_user_not_found() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();

        let goal_name = Symbol::new(&env, "nouser");
        let target = 5000i128;
        let initial = 1000i128;

        client.create_goal_save(&user, &goal_name, &target, &initial);
    }

    #[test]
    fn test_goal_create_with_protocol_fee() {
        let (env, client, _admin) = setup_admin_env();
        let user = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);
        assert!(client.try_set_fee_recipient(&treasury).is_ok());
        assert!(client.try_set_protocol_fee_bps(&500).is_ok()); // 5%

        let goal_name = Symbol::new(&env, "vacation");
        let target = 10_000i128;
        let initial = 5_000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);

        let goal_save = client.get_goal_save_detail(&goal_id);
        // Net = 5,000 - 250 = 4,750
        assert_eq!(goal_save.current_amount, 4_750);
        assert_eq!(client.get_protocol_fee_balance(&treasury), 250);
    }

    #[test]
    fn test_goal_deposit_with_protocol_fee() {
        let (env, client, _admin) = setup_admin_env();
        let user = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);
        assert!(client.try_set_fee_recipient(&treasury).is_ok());
        assert!(client.try_set_protocol_fee_bps(&300).is_ok()); // 3%

        let goal_name = Symbol::new(&env, "house");
        let target = 10_000i128;
        let initial = 2_000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        // Initial: 2,000 - 60 = 1,940
        assert_eq!(client.get_protocol_fee_balance(&treasury), 60);

        client.deposit_to_goal_save(&user, &goal_id, &3_000);
        // Deposit: 3,000 - 90 = 2,910
        // Total in goal: 1,940 + 2,910 = 4,850
        // Total fees: 60 + 90 = 150

        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.current_amount, 4_850);
        assert_eq!(client.get_protocol_fee_balance(&treasury), 150);
    }

    #[test]
    fn test_goal_withdraw_with_protocol_fee() {
        let (env, client, _admin) = setup_admin_env();
        let user = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);
        assert!(client.try_set_fee_recipient(&treasury).is_ok());
        assert!(client.try_set_protocol_fee_bps(&250).is_ok()); // 2.5%

        let goal_name = Symbol::new(&env, "laptop");
        let target = 4_000i128;
        let initial = 5_000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        // Initial: 5,000 - 125 = 4,875 (exceeds target of 4,000, so completed)
        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.current_amount, 4_875);
        assert!(goal_save.is_completed);
        assert_eq!(client.get_protocol_fee_balance(&treasury), 125);

        let amount = client.withdraw_completed_goal_save(&user, &goal_id);
        // Withdrawal: 4,875 - 121 = 4,754 (fee rounded down)
        assert_eq!(amount, 4_754);
        // Total fees: 125 + 121 = 246
        assert_eq!(client.get_protocol_fee_balance(&treasury), 246);
    }

    #[test]
    fn test_goal_zero_protocol_fee() {
        let (env, client) = setup_test_env();
        let user = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);

        let goal_name = Symbol::new(&env, "car");
        let target = 5_000i128;
        let initial = 5_000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.current_amount, 5_000);
        assert!(goal_save.is_completed);

        let amount = client.withdraw_completed_goal_save(&user, &goal_id);
        assert_eq!(amount, 5_000);
    }

    #[test]
    fn test_goal_fee_calculation_correctness() {
        let (env, client, _admin) = setup_admin_env();
        let user = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);
        assert!(client.try_set_fee_recipient(&treasury).is_ok());
        assert!(client.try_set_protocol_fee_bps(&1000).is_ok()); // 10%

        let goal_name = Symbol::new(&env, "test");
        let target = 10_000i128;
        let initial = 1_000i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        // Fee = 1,000 * 10% = 100
        // Net = 900
        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.current_amount, 900);
        assert_eq!(client.get_protocol_fee_balance(&treasury), 100);
    }

    #[test]
    fn test_goal_small_amount_fee_edge_case() {
        let (env, client, _admin) = setup_admin_env();
        let user = Address::generate(&env);
        let treasury = Address::generate(&env);

        env.mock_all_auths();
        client.initialize_user(&user);
        assert!(client.try_set_fee_recipient(&treasury).is_ok());
        assert!(client.try_set_protocol_fee_bps(&100).is_ok()); // 1%

        let goal_name = Symbol::new(&env, "small");
        let target = 1_000i128;
        let initial = 50i128;

        let goal_id = client.create_goal_save(&user, &goal_name, &target, &initial);
        // Fee = floor(50 * 100 / 10000) = 0
        // Net = 50
        let goal_save = client.get_goal_save_detail(&goal_id);
        assert_eq!(goal_save.current_amount, 50);
        assert_eq!(client.get_protocol_fee_balance(&treasury), 0);
    }
}
