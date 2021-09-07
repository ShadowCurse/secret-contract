use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HandleMsg {
    DepositTo {
        to: HumanAddr,
        value: Uint128,
    },
    BurnFrom {
        from: HumanAddr,
        value: Uint128,
    },
    Transfer {
        to: HumanAddr,
        value: Uint128,
    },
    TransferFrom {
        from: HumanAddr,
        to: HumanAddr,
        value: Uint128,
    },
    Approve {
        spender: HumanAddr,
        value: Uint128,
    },
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HandleResult {
    DepositTo {
        status: Status,
    },
    BurnFrom {
        status: Status,
    },
    Transfer {
        status: Status,
    },
    TransferFrom {
        status: Status,
    },
    Approve {
        status: Status,
    },
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
        value: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QueryMsg {
    Name {},
    Symbol {},
    Decimals {},
    BalanceOf { address: HumanAddr },
    TotalSupply {},
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QueryResult {
    Name { name: String },
    Symbol { symbol: String },
    Decimals { decimals: u8 },
    BalanceOf { balance: Uint128 },
    TotalSupply { total_supply: Uint128 },
    Owner { owner: String },
}
