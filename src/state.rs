use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use serde::de::DeserializeOwned;
use std::any::type_name;
use std::convert::TryFrom;

pub const NAMESPACE_STORAGE: &[u8] = b"config";
pub static KEY_CONSTANTS: &[u8] = b"constants";
pub static KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";

pub const NAMESPACE_BALANCES: &[u8] = b"balances";
pub const NAMESPACE_ALLOWANCES: &[u8] = b"allowancws";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Constants {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub owner: CanonicalAddr,
}

type TotalSupply = u128;

fn serialize<T: Serialize>(value: &T) -> StdResult<Vec<u8>> {
    bincode::serialize::<T>(value).map_err(|e| StdError::serialize_err(type_name::<T>(), e))
}

fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> StdResult<T> {
    bincode::deserialize::<T>(bytes).map_err(|e| StdError::serialize_err(type_name::<T>(), e))
}

fn slice_to_u128(bytes: &[u8]) -> StdResult<u128> {
    match <[u8; 16]>::try_from(bytes) {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "corrupted data, can not convert to u128",
        )),
    }
}

pub struct ContractStorage<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> ContractStorage<'a, S> {
    pub fn from_storage(storage: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(NAMESPACE_STORAGE, storage),
        }
    }

    fn as_readonly(&self) -> ReadOnlyContractStorageImpl<PrefixedStorage<S>> {
        ReadOnlyContractStorageImpl(&self.storage)
    }

    pub fn constants(&self) -> StdResult<Constants> {
        self.as_readonly().constants()
    }

    pub fn set_constants(&mut self, constants: &Constants) -> StdResult<()> {
        self.storage
            .set(KEY_CONSTANTS, serialize(constants)?.as_ref());
        Ok(())
    }

    pub fn total_supply(&self) -> StdResult<TotalSupply> {
        self.as_readonly().total_supply()
    }

    pub fn set_total_supply(&mut self, value: u128) -> StdResult<()> {
        self.storage.set(KEY_TOTAL_SUPPLY, &value.to_be_bytes()); //serialize(&value)?.as_ref());
        Ok(())
    }
}

pub struct ReadOnlyContractStorage<'a, S: Storage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: Storage> ReadOnlyContractStorage<'a, S> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(NAMESPACE_STORAGE, storage),
        }
    }

    fn as_readonly(&self) -> ReadOnlyContractStorageImpl<ReadonlyPrefixedStorage<S>> {
        ReadOnlyContractStorageImpl(&self.storage)
    }

    pub fn constants(&self) -> StdResult<Constants> {
        self.as_readonly().constants()
    }

    pub fn total_supply(&self) -> StdResult<TotalSupply> {
        self.as_readonly().total_supply()
    }
}

struct ReadOnlyContractStorageImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadOnlyContractStorageImpl<'a, S> {
    pub fn constants(&self) -> StdResult<Constants> {
        let bytes = self
            .0
            .get(KEY_CONSTANTS)
            .ok_or(StdError::generic_err("no constants in storage"))?;
        deserialize(&bytes)
    }

    pub fn total_supply(&self) -> StdResult<TotalSupply> {
        let bytes = self
            .0
            .get(KEY_TOTAL_SUPPLY)
            .ok_or(StdError::generic_err("no constants in storage"))?;
        slice_to_u128(&bytes)
    }
}

pub struct Balances<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> Balances<'a, S> {
    pub fn from_storage(storage: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(NAMESPACE_BALANCES, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyBalancesImpl<PrefixedStorage<S>> {
        ReadonlyBalancesImpl(&self.storage)
    }

    pub fn balance(&self, address: &CanonicalAddr) -> u128 {
        self.as_readonly().balance(address)
    }

    pub fn set_balance(&mut self, address: &CanonicalAddr, value: u128) {
        self.storage.set(address.as_slice(), &value.to_be_bytes());
    }
}

pub struct ReadOnlyBalances<'a, S: Storage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: Storage> ReadOnlyBalances<'a, S> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(NAMESPACE_BALANCES, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyBalancesImpl<ReadonlyPrefixedStorage<S>> {
        ReadonlyBalancesImpl(&self.storage)
    }

    pub fn balance(&self, address: &CanonicalAddr) -> u128 {
        self.as_readonly().balance(address)
    }
}

struct ReadonlyBalancesImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadonlyBalancesImpl<'a, S> {
    pub fn balance(&self, account: &CanonicalAddr) -> u128 {
        match self.0.get(account.as_slice()) {
            Some(balance_bytes) => slice_to_u128(&balance_bytes).unwrap(),
            None => 0,
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, Default, JsonSchema)]
pub struct Allowance {
    pub amount: u128,
}

pub fn get_allowance<S: Storage>(
    storage: &S,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
) -> StdResult<Allowance> {
    let owner_storage =
        ReadonlyPrefixedStorage::multilevel(&[NAMESPACE_ALLOWANCES, owner.as_slice()], storage);
    match owner_storage.get(spender.as_slice()) {
        Some(bytes) => deserialize(&bytes)?,
        None => Ok(Allowance { amount: 0 }),
    }
}

pub fn set_allowance<S: Storage>(
    storage: &mut S,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
    allowance: Allowance,
) -> StdResult<()> {
    let mut owner_storage =
        PrefixedStorage::multilevel(&[NAMESPACE_ALLOWANCES, owner.as_slice()], storage);
    owner_storage.set(spender.as_slice(), serialize(&allowance)?.as_ref());
    Ok(())
}
