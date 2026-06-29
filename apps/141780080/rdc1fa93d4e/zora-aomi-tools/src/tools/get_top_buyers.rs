use std::collections::HashMap;

use anyhow::Result;

use crate::{
    client::{SwapEdge, ZoraClient},
    types::{TopBuyer, TopBuyersInput},
};

pub async fn get_top_buyers(input: TopBuyersInput, client: &ZoraClient) -> Result<Vec<TopBuyer>> {
    let first = input.top_n.saturating_mul(20).max(100);
    let swaps = client
        .get_coin_swaps(&input.address, input.chain, first as u32, None)
        .await?;

    let edges = swaps
        .data
        .zora20_token
        .and_then(|token| token.swap_activities)
        .map(|activities| activities.edges)
        .unwrap_or_default();

    Ok(aggregate_top_buyers(edges, input.top_n as usize))
}

pub fn aggregate_top_buyers(edges: Vec<SwapEdge>, top_n: usize) -> Vec<TopBuyer> {
    let mut totals: HashMap<String, (String, u32)> = HashMap::new();

    for edge in edges {
        let node = edge.node;
        if !node.activity_type.eq_ignore_ascii_case("BUY") {
            continue;
        }

        let Some(address) = node.sender_address else {
            continue;
        };
        let Some(amount) = node.coin_amount else {
            continue;
        };

        let entry = totals.entry(address).or_insert(("0".to_string(), 0));
        entry.0 = add_amounts(&entry.0, &amount);
        entry.1 += 1;
    }

    let mut ranked = totals
        .into_iter()
        .map(|(address, (total_bought, trade_count))| (address, total_bought, trade_count))
        .collect::<Vec<_>>();

    ranked.sort_by(|a, b| compare_amounts(&b.1, &a.1).then_with(|| a.0.cmp(&b.0)));

    ranked
        .into_iter()
        .take(top_n)
        .enumerate()
        .map(|(index, (address, total_bought, trade_count))| TopBuyer {
            rank: (index + 1) as u32,
            address,
            total_bought,
            trade_count,
        })
        .collect()
}

fn add_amounts(left: &str, right: &str) -> String {
    let (left_int, left_frac) = split_amount(left);
    let (right_int, right_frac) = split_amount(right);
    let frac_len = left_frac.len().max(right_frac.len());
    let left_digits = format!("{}{}", left_int, pad_right(left_frac, frac_len));
    let right_digits = format!("{}{}", right_int, pad_right(right_frac, frac_len));

    let mut carry = 0u8;
    let mut sum = Vec::new();
    let mut left_iter = left_digits.bytes().rev();
    let mut right_iter = right_digits.bytes().rev();

    loop {
        let left_digit = left_iter.next().map(|byte| byte - b'0');
        let right_digit = right_iter.next().map(|byte| byte - b'0');

        if left_digit.is_none() && right_digit.is_none() && carry == 0 {
            break;
        }

        let digit_sum = left_digit.unwrap_or(0) + right_digit.unwrap_or(0) + carry;
        sum.push((digit_sum % 10 + b'0') as char);
        carry = digit_sum / 10;
    }

    let mut digits = sum.into_iter().rev().collect::<String>();
    if frac_len > 0 {
        if digits.len() <= frac_len {
            digits = format!("{}{}", "0".repeat(frac_len + 1 - digits.len()), digits);
        }
        let point = digits.len() - frac_len;
        digits.insert(point, '.');
    }

    normalize_amount(&digits)
}

fn compare_amounts(left: &str, right: &str) -> std::cmp::Ordering {
    let (left_int, left_frac) = split_amount(left);
    let (right_int, right_frac) = split_amount(right);

    left_int
        .len()
        .cmp(&right_int.len())
        .then_with(|| left_int.cmp(right_int))
        .then_with(|| {
            let frac_len = left_frac.len().max(right_frac.len());
            pad_right(left_frac, frac_len).cmp(&pad_right(right_frac, frac_len))
        })
}

fn split_amount(amount: &str) -> (&str, &str) {
    let amount = amount.trim();
    amount.split_once('.').unwrap_or((amount, ""))
}

fn pad_right(value: &str, len: usize) -> String {
    format!("{value:0<len$}")
}

fn normalize_amount(amount: &str) -> String {
    let (int, frac) = split_amount(amount);
    let int = int.trim_start_matches('0');
    let int = if int.is_empty() { "0" } else { int };
    let frac = frac.trim_end_matches('0');

    if frac.is_empty() {
        int.to_string()
    } else {
        format!("{int}.{frac}")
    }
}

#[cfg(test)]
mod tests {
    use crate::client::{SwapEdge, SwapNode};

    use super::aggregate_top_buyers;

    fn edge(activity_type: &str, address: &str, amount: &str) -> SwapEdge {
        SwapEdge {
            node: SwapNode {
                activity_type: activity_type.to_string(),
                coin_amount: Some(amount.to_string()),
                sender_address: Some(address.to_string()),
                block_timestamp: None,
            },
        }
    }

    #[test]
    fn aggregates_only_buys_and_ranks_by_amount() {
        let ranked = aggregate_top_buyers(
            vec![
                edge("BUY", "0xaaa", "1.5"),
                edge("SELL", "0xbbb", "100"),
                edge("BUY", "0xccc", "3"),
                edge("BUY", "0xaaa", "2.25"),
            ],
            2,
        );

        assert_eq!(ranked[0].address, "0xaaa");
        assert_eq!(ranked[0].total_bought, "3.75");
        assert_eq!(ranked[0].trade_count, 2);
        assert_eq!(ranked[1].address, "0xccc");
    }

    #[test]
    fn aggregates_large_amounts_without_float_precision_loss() {
        let ranked = aggregate_top_buyers(
            vec![
                edge("BUY", "0xaaa", "900719925474099312345"),
                edge("BUY", "0xaaa", "7.55"),
                edge("BUY", "0xbbb", "900719925474099312346"),
            ],
            2,
        );

        assert_eq!(ranked[0].address, "0xaaa");
        assert_eq!(ranked[0].total_bought, "900719925474099312352.55");
        assert_eq!(ranked[1].address, "0xbbb");
    }
}
