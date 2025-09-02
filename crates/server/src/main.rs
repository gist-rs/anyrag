#[tokio::main]
async fn main() -> anyhow::Result<()> {
    anyrag_server::start().await
}
