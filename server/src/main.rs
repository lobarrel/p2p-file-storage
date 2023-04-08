use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::net::{TcpListener};

#[tokio::main]
async fn main() -> io::Result<()> {
    // let mut f = File::open("./data.txt").await?;
    // let mut buffer = Vec::new();

    // // read the whole file
    // f.read_to_end(&mut buffer).await?;
    // println!("{:?}", buffer);

    // let mut f2 = File::create("./output.txt").await?;
    // f2.write_all(&mut buffer).await?;
    // Ok(())

    let listener = TcpListener::bind("localhost:8080").await.unwrap();

    loop{
        let (mut socket, _) = listener.accept().await.unwrap();
        
        
        tokio::spawn(async move{
            println!("Connection opened");
           
            let mut f = File::create("./output.txt").await.unwrap();
            let mut buf = [0u8; 1];

            let (mut reader, _) = socket.split();
            
            loop {
                match reader.read(&mut buf).await{
                    Ok(0) => return,
                    Ok(_n) =>{
                            f.write_all(&mut buf).await.unwrap();
                        },
                    Err(e) => println!("{}",e)
                    };
                }  
        });
    }
}