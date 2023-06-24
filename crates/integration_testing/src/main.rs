use anyhow::Result;

mod reddit_dump_importer;

#[tokio::main]
async fn main() -> Result<()> {
  Ok(reddit_dump_importer::go().await)
}
