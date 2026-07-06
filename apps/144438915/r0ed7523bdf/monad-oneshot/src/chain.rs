//! Monad mainnet constants. Just two things: chain id and WMON.
//!
//! WMON is Monad's wrapped-native ERC-20. It is part of the chain
//! infrastructure — every Monad DEX uses it. Verified on-chain 2026-07-04
//! (`symbol()` returns "WMON"); source of truth is
//! https://docs.monad.xyz/developer-essentials/network-information.

pub const MONAD_CHAIN_ID: u64 = 143;
pub const MONAD_RPC: &str = "https://rpc.monad.xyz";
pub const WMON_ADDRESS: &str = "0x3bd359C1119dA7Da1D913D1C4D2B7c461115433A";
