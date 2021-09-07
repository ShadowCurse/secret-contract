use cosmwasm_std::{
    to_binary, Api, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier, QueryResponse,
    StdError, StdResult, Storage, Uint128,
};

use crate::msg::{HandleMsg, HandleResult, InitMsg, QueryMsg, QueryResult, Status};
use crate::state::{
    get_allowance, set_allowance, Allowance, Balances, Constants, ContractStorage,
    ReadOnlyBalances, ReadOnlyContractStorage,
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let constants = Constants {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        owner: deps.api.canonical_address(&env.message.sender)?,
    };

    let mut storage = ContractStorage::from_storage(&mut deps.storage);
    storage.set_constants(&constants)?;
    storage.set_total_supply(0)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::DepositTo { to, value } => deposit_to(deps, env, to, value),
        HandleMsg::BurnFrom { from, value } => burn_from(deps, env, from, value),
        HandleMsg::Transfer { to, value } => transfer(deps, env, to, value),
        HandleMsg::TransferFrom { from, to, value } => transfer_from(deps, env, from, to, value),
        HandleMsg::Approve { spender, value } => approve(deps, env, spender, value),
        HandleMsg::Allowance { owner, spender } => allowance(deps, owner, spender),
    }
}

fn deposit_to<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    to: HumanAddr,
    value: Uint128,
) -> StdResult<HandleResponse> {
    if value.is_zero() {
        return Err(StdError::generic_err("Can not deposit zero tokens"));
    }

    let mut storage = ContractStorage::from_storage(&mut deps.storage);

    let sender = deps.api.canonical_address(&env.message.sender)?;
    let contract_owner = storage.constants()?.owner;
    if sender != contract_owner {
        return Err(StdError::generic_err(
            "Only contract owner can deposit tokens",
        ));
    }

    let total_supply = storage.total_supply()?;
    if let Some(new_total) = total_supply.checked_add(value.u128()) {
        storage.set_total_supply(new_total)?;
    } else {
        return Err(StdError::generic_err("Total supply overflow"));
    }

    let account_owner = deps.api.canonical_address(&to)?;
    let mut balances = Balances::from_storage(&mut deps.storage);
    let sender_balance = balances.balance(&account_owner);
    if let Some(new_balance) = sender_balance.checked_add(value.u128()) {
        balances.set_balance(&account_owner, new_balance);
    } else {
        return Err(StdError::generic_err("Account balance overflow"));
    }

    let res = HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::DepositTo {
            status: Status::Success,
        })?),
    };
    Ok(res)
}

fn burn_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: HumanAddr,
    value: Uint128,
) -> StdResult<HandleResponse> {
    if value.is_zero() {
        return Err(StdError::generic_err("Can not burn zero tokens"));
    }

    let sender = deps.api.canonical_address(&env.message.sender)?;
    let account_owner = deps.api.canonical_address(&from)?;
    {
        let storage = ReadOnlyContractStorage::from_storage(&deps.storage);
        let contract_owner = storage.constants()?.owner;
        if sender != contract_owner && sender != account_owner {
            return Err(StdError::generic_err(
                "Only contract owner or account owner can burn tokens",
            ));
        }
    }

    let mut balances = Balances::from_storage(&mut deps.storage);
    let sender_balance = balances.balance(&account_owner);
    if let Some(new_balance) = sender_balance.checked_sub(value.u128()) {
        balances.set_balance(&account_owner, new_balance);
    } else {
        return Err(StdError::generic_err("Account balance underflow"));
    }

    let mut storage = ContractStorage::from_storage(&mut deps.storage);
    let total_supply = storage.total_supply()?;
    if let Some(new_total) = total_supply.checked_sub(value.u128()) {
        storage.set_total_supply(new_total)?;
    } else {
        return Err(StdError::generic_err("Total supply underflow"));
    }

    let res = HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::BurnFrom {
            status: Status::Success,
        })?),
    };
    Ok(res)
}

fn transfer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    to: HumanAddr,
    value: Uint128,
) -> StdResult<HandleResponse> {
    if value.is_zero() {
        return Err(StdError::generic_err("Can not transfer zero tokens"));
    }

    let sender = deps.api.canonical_address(&env.message.sender)?;
    let recipient = deps.api.canonical_address(&to)?;

    if sender == recipient {
        return Err(StdError::generic_err("Can not sent tokens to self"));
    }

    let mut balances = Balances::from_storage(&mut deps.storage);
    let sender_balance = balances.balance(&sender);
    let recipient_balance = balances.balance(&recipient);
    if let Some(new_sender_balance) = sender_balance.checked_sub(value.u128()) {
        if let Some(new_recipient_balance) = recipient_balance.checked_add(value.u128()) {
            balances.set_balance(&sender, new_sender_balance);
            balances.set_balance(&recipient, new_recipient_balance);
        } else {
            return Err(StdError::generic_err("Recipient balance overflow"));
        }
    } else {
        return Err(StdError::generic_err("Sender balance underflow"));
    }

    let res = HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::Transfer {
            status: Status::Success,
        })?),
    };
    Ok(res)
}

fn transfer_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: HumanAddr,
    to: HumanAddr,
    value: Uint128,
) -> StdResult<HandleResponse> {
    if value.is_zero() {
        return Err(StdError::generic_err("Can not transfer zero tokens"));
    }

    let sender = deps.api.canonical_address(&env.message.sender)?;
    let account_owner = deps.api.canonical_address(&from)?;
    let recipient = deps.api.canonical_address(&to)?;

    if sender == account_owner {
        return Err(StdError::generic_err(
            "To transfer tokens from your own account use transfer function",
        ));
    }

    if account_owner == recipient {
        return Err(StdError::generic_err(
            "Can not transfer tokens: from and to addresses are same",
        ));
    }

    let allowance = get_allowance(&deps.storage, &account_owner, &sender)?;
    let mut balances = Balances::from_storage(&mut deps.storage);
    let account_balance = balances.balance(&account_owner);
    let recipient_balance = balances.balance(&recipient);

    if let Some(new_allowance) = allowance.amount.checked_sub(value.u128()) {
        if let Some(new_account_balance) = account_balance.checked_sub(value.u128()) {
            if let Some(new_recipient_balance) = recipient_balance.checked_add(value.u128()) {
                balances.set_balance(&account_owner, new_account_balance);
                balances.set_balance(&recipient, new_recipient_balance);
                set_allowance(
                    &mut deps.storage,
                    &account_owner,
                    &sender,
                    Allowance {
                        amount: new_allowance,
                    },
                )?;
            } else {
                return Err(StdError::generic_err("Recipient balance overflow"));
            }
        } else {
            return Err(StdError::generic_err("Account balance underflow"));
        }
    } else {
        return Err(StdError::generic_err("Not enough allowance"));
    }

    let res = HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::TransferFrom {
            status: Status::Success,
        })?),
    };
    Ok(res)
}

fn approve<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    value: Uint128,
) -> StdResult<HandleResponse> {
    if value.is_zero() {
        return Err(StdError::generic_err("Can not approve zero tokens"));
    }

    let sender = deps.api.canonical_address(&env.message.sender)?;
    let spender = deps.api.canonical_address(&spender)?;

    if sender == spender {
        return Err(StdError::generic_err("Can not approve to self"));
    }

    let mut allowance = get_allowance(&deps.storage, &sender, &spender)?;
    allowance.amount = allowance.amount.saturating_add(value.u128());

    set_allowance(&mut deps.storage, &sender, &spender, allowance)?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::Approve {
            status: Status::Success,
        })?),
    };
    Ok(res)
}

fn allowance<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    owner: HumanAddr,
    spender: HumanAddr,
) -> StdResult<HandleResponse> {
    let owner_address = deps.api.canonical_address(&owner)?;
    let spender_address = deps.api.canonical_address(&spender)?;

    let allowance = get_allowance(&deps.storage, &owner_address, &spender_address)?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleResult::Allowance {
            owner,
            spender,
            value: Uint128(allowance.amount),
        })?),
    };
    Ok(res)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Name {} => query_name(deps),
        QueryMsg::Symbol {} => query_symbol(deps),
        QueryMsg::Decimals {} => query_decimals(deps),
        QueryMsg::TotalSupply {} => query_total_supply(deps),
        QueryMsg::BalanceOf { address } => query_balance_of(deps, address),
    }
}

fn query_name<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<QueryResponse> {
    to_binary(&QueryResult::Name {
        name: ReadOnlyContractStorage::from_storage(&deps.storage)
            .constants()?
            .name,
    })
}

fn query_symbol<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<QueryResponse> {
    to_binary(&QueryResult::Symbol {
        symbol: ReadOnlyContractStorage::from_storage(&deps.storage)
            .constants()?
            .symbol,
    })
}

fn query_decimals<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<QueryResponse> {
    to_binary(&QueryResult::Decimals {
        decimals: ReadOnlyContractStorage::from_storage(&deps.storage)
            .constants()?
            .decimals,
    })
}

fn query_total_supply<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<QueryResponse> {
    to_binary(&QueryResult::TotalSupply {
        total_supply: ReadOnlyContractStorage::from_storage(&deps.storage)
            .total_supply()?
            .into(),
    })
}

fn query_balance_of<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
) -> StdResult<QueryResponse> {
    let address = deps.api.canonical_address(&address)?;
    let balances = ReadOnlyBalances::from_storage(&deps.storage);
    let balance = balances.balance(&address);
    to_binary(&QueryResult::BalanceOf {
        balance: Uint128::from(balance),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{coins, from_binary};

    fn initialize() -> Extern<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            name: "test".to_string(),
            symbol: "!@#$".to_string(),
            decimals: 69,
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn initialization() {
        let deps = initialize();

        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(0, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn handle_deposit_to() {
        let mut deps = initialize();
        let address = HumanAddr::from("address");
        let handle_msg = HandleMsg::DepositTo {
            to: address.clone(),
            value: Uint128(69),
        };
        match handle(&mut deps, mock_env("creator", &[]), handle_msg) {
            Ok(_) => {}
            Err(e) => panic!("error: {:?}", e),
        }

        // checking new balance
        let res = query(&deps, QueryMsg::BalanceOf { address }).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::BalanceOf { balance } => assert_eq!(69, balance.u128()),
            _ => panic!("unexpected"),
        }

        // checking new total supply
        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(69, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn handle_deposit_to_invalid_sender() {
        let mut deps = initialize();
        let address = HumanAddr::from("address");
        let handle_msg = HandleMsg::DepositTo {
            to: address.clone(),
            value: Uint128(69),
        };
        match handle(&mut deps, mock_env("bob", &[]), handle_msg) {
            Ok(_) => panic!("should have failed"),
            _ => {}
        }

        // checking balance
        let res = query(&deps, QueryMsg::BalanceOf { address }).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::BalanceOf { balance } => assert_eq!(0, balance.u128()),
            _ => panic!("unexpected"),
        }

        // checking new total supply
        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(0, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn handle_deposit_to_invalid_founds() {
        let mut deps = initialize();
        let address = HumanAddr::from("address");
        let handle_msg = HandleMsg::DepositTo {
            to: address.clone(),
            value: Uint128(0),
        };
        match handle(&mut deps, mock_env("bob", &[]), handle_msg) {
            Ok(_) => panic!("should have failed"),
            _ => {}
        }

        // checking balance
        let res = query(&deps, QueryMsg::BalanceOf { address }).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::BalanceOf { balance } => assert_eq!(0, balance.u128()),
            _ => panic!("unexpected"),
        }

        // checking new total supply
        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(0, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn handle_burn_from() {
        let mut deps = initialize();
        let address = HumanAddr::from("address");
        let handle_msg = HandleMsg::DepositTo {
            to: address.clone(),
            value: Uint128(69),
        };
        match handle(&mut deps, mock_env("creator", &[]), handle_msg) {
            Ok(_) => {}
            Err(e) => panic!("error: {:?}", e),
        }

        let handle_msg = HandleMsg::BurnFrom {
            from: address.clone(),
            value: Uint128(9),
        };
        match handle(&mut deps, mock_env("creator", &[]), handle_msg) {
            Ok(_) => {}
            Err(e) => panic!("error: {:?}", e),
        }

        // checking new balance
        let res = query(&deps, QueryMsg::BalanceOf { address }).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::BalanceOf { balance } => assert_eq!(60, balance.u128()),
            _ => panic!("unexpected"),
        }

        // checking new total supply
        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(60, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn handle_burn_from_owner() {
        let mut deps = initialize();
        let address = HumanAddr::from("address");
        let handle_msg = HandleMsg::DepositTo {
            to: address.clone(),
            value: Uint128(69),
        };
        match handle(&mut deps, mock_env("creator", &[]), handle_msg) {
            Ok(_) => {}
            Err(e) => panic!("error: {:?}", e),
        }

        let handle_msg = HandleMsg::BurnFrom {
            from: address.clone(),
            value: Uint128(9),
        };
        match handle(&mut deps, mock_env(address.clone(), &[]), handle_msg) {
            Ok(_) => {}
            Err(e) => panic!("error: {:?}", e),
        }

        // checking new balance
        let res = query(&deps, QueryMsg::BalanceOf { address }).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::BalanceOf { balance } => assert_eq!(60, balance.u128()),
            _ => panic!("unexpected"),
        }

        // checking new total supply
        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(60, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn handle_burn_from_invalid_sender() {
        let mut deps = initialize();
        let address = HumanAddr::from("address");
        let handle_msg = HandleMsg::DepositTo {
            to: address.clone(),
            value: Uint128(69),
        };
        match handle(&mut deps, mock_env("creator", &[]), handle_msg) {
            Ok(_) => {}
            Err(e) => panic!("error: {:?}", e),
        }

        let handle_msg = HandleMsg::BurnFrom {
            from: address.clone(),
            value: Uint128(9),
        };
        match handle(&mut deps, mock_env("bob", &[]), handle_msg) {
            Ok(_) => panic!("should have failed"),
            Err(_) => {}
        }

        // checking balance
        let res = query(&deps, QueryMsg::BalanceOf { address }).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::BalanceOf { balance } => assert_eq!(69, balance.u128()),
            _ => panic!("unexpected"),
        }

        // checking new total supply
        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(69, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn handle_burn_from_invalid_founds() {
        let mut deps = initialize();
        let address = HumanAddr::from("address");
        let handle_msg = HandleMsg::DepositTo {
            to: address.clone(),
            value: Uint128(0),
        };
        match handle(&mut deps, mock_env("creator", &[]), handle_msg) {
            Ok(_) => panic!("should have failed"),
            Err(_) => {}
        }

        // checking balance
        let res = query(&deps, QueryMsg::BalanceOf { address }).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::BalanceOf { balance } => assert_eq!(0, balance.u128()),
            _ => panic!("unexpected"),
        }

        // checking new total supply
        let res = query(&deps, QueryMsg::TotalSupply {}).unwrap();
        match from_binary(&res).unwrap() {
            QueryResult::TotalSupply { total_supply } => assert_eq!(0, total_supply.u128()),
            _ => panic!("unexpected"),
        }
    }
}
