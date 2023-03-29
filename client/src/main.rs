use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::fs::File;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> io::Result<()> {
    // let mut f2 = File::create("./output.txt").await?;
    // f2.write_all(&mut buffer).await?;
    // Ok(())

    
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();

    let mut f = File::open("./data.txt").await?;
    let mut buffer = Vec::new();

    // read the whole file
    f.read_to_end(&mut buffer).await?;
    println!("{:?}", buffer);

    stream.write_all(&mut buffer).await?;


    Ok(())

    
}
