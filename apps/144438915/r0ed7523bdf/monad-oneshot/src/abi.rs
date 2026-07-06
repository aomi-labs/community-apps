//! Minimal ABI helpers. No crypto dependency — selectors are hardcoded.

use num_bigint::BigUint;
use num_traits::Num;

// Selectors = first 4 bytes of keccak256(signature).
pub const SEL_BALANCE_OF: [u8; 4] = [0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
pub const SEL_DEPOSIT: [u8; 4] = [0xd0, 0xe3, 0x0d, 0xb0];    // deposit()
pub const SEL_WITHDRAW: [u8; 4] = [0x2e, 0x1a, 0x7d, 0x4d];   // withdraw(uint256)

pub fn parse_address(s: &str) -> Result<[u8; 20], String> {
    let trimmed = s.trim_start_matches("0x").trim_start_matches("0X");
    if trimmed.len() != 40 {
        return Err(format!("invalid address length: {s}"));
    }
    let bytes = hex::decode(trimmed).map_err(|e| format!("invalid address hex: {e}"))?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

pub fn word_address(addr: &[u8; 20]) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[12..].copy_from_slice(addr);
    w
}

pub fn word_uint_decimal(decimal: &str) -> Result<[u8; 32], String> {
    let n = BigUint::from_str_radix(decimal.trim(), 10)
        .map_err(|e| format!("invalid uint `{decimal}`: {e}"))?;
    let raw = n.to_bytes_be();
    if raw.len() > 32 {
        return Err(format!("uint overflows 32 bytes: {decimal}"));
    }
    let mut w = [0u8; 32];
    w[32 - raw.len()..].copy_from_slice(&raw);
    Ok(w)
}

pub fn to_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

pub fn decode_uint_word(hex_response: &str) -> Result<BigUint, String> {
    let trimmed = hex_response.trim_start_matches("0x").trim_start_matches("0X");
    if trimmed.is_empty() {
        return Ok(BigUint::from(0u32));
    }
    // eth_getBalance returns minimal hex (e.g. "0x0", "0xde0b6b3a7640000") which
    // can have an odd digit count; hex::decode needs even length, so left-pad.
    let padded = if trimmed.len() % 2 == 1 {
        format!("0{trimmed}")
    } else {
        trimmed.to_string()
    };
    let bytes = hex::decode(&padded).map_err(|e| format!("bad hex from RPC: {e}"))?;
    Ok(BigUint::from_bytes_be(&bytes))
}

// ─── Calldata builders ──────────────────────────────────────────────────────

pub fn balance_of(owner: &[u8; 20]) -> Vec<u8> {
    let mut data = Vec::with_capacity(36);
    data.extend_from_slice(&SEL_BALANCE_OF);
    data.extend_from_slice(&word_address(owner));
    data
}

pub fn deposit() -> Vec<u8> {
    SEL_DEPOSIT.to_vec()
}

pub fn withdraw(amount_wei: &str) -> Result<Vec<u8>, String> {
    let mut data = Vec::with_capacity(36);
    data.extend_from_slice(&SEL_WITHDRAW);
    data.extend_from_slice(&word_uint_decimal(amount_wei)?);
    Ok(data)
}
