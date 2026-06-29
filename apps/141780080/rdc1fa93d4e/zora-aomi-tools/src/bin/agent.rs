#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(command) = args.first() {
        match command.as_str() {
            "query" | "ask" => {
                args.remove(0);
                if args.is_empty() {
                    anyhow::bail!("missing query text");
                }

                let query = args.join(" ");
                let intent = zora_aomi_tools::agent::parse_user_input(&query)
                    .ok_or_else(|| anyhow::anyhow!("Could not understand query"))?;
                let output = zora_aomi_tools::agent::execute_intent(&intent)
                    .await
                    .map_err(anyhow::Error::msg)?;

                println!("{}", output.replace("\\n", "\n"));
                return Ok(());
            }
            "serve" | "server" => {
                args.remove(0);
            }
            _ => {}
        }
    }

    zora_aomi_tools::server::run_from_env().await
}
