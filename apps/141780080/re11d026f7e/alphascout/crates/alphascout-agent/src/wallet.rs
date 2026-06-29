use ethers::signers::LocalWallet;
use std::str::FromStr;

pub fn load_wallet() -> LocalWallet {
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY missing");

    LocalWallet::from_str(&private_key).unwrap()
}
