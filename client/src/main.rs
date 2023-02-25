use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut f = File::open("./data.txt").await?;
    let mut buffer = Vec::new();

    // read the whole file
    f.read_to_end(&mut buffer).await?;
    println!("{:?}", buffer);

    let mut f2 = File::create("./output.txt").await?;
    f2.write_all(&mut buffer).await?;
    Ok(())
}