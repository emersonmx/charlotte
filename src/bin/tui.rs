#[tokio::main]
async fn main() -> anyhow::Result<()> {
    charlotte::serve().await?;
    Ok(())
}
