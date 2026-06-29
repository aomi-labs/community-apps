use ethers::providers::{Http, Provider};

pub fn base_provider() -> Provider<Http> {
    let rpc = std::env::var("BASE_RPC_URL").unwrap();

    Provider::<Http>::try_from(rpc).unwrap()
}
