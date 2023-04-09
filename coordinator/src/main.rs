use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::fs::File;
use tokio::net::{TcpListener};
use std::{str, fs};
use std::sync::{Arc, Mutex};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct Provider {
    ip_addr: String,
    btc_addr: String,
}

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
    let db = Arc::new(Mutex::new(Vec::<Provider>::new()));
    //let mut f = File::create("./providers.json").await.unwrap();
    

    loop{
        let (mut socket, _) = listener.accept().await.unwrap();
        
        let db = db.clone();
        tokio::spawn(async move{
            println!("Connection opened");
           
            
            let mut buf = [0u8; 54];

            let (mut reader, _) = socket.split();
            
            
            match reader.read(&mut buf).await{
                Ok(0) => return,
                Ok(_n) =>{
                        //f.write_all(&mut buf).await.unwrap();
                        let s = String::from_utf8_lossy(&buf);
                        println!("{}", s);
                        let parts: Vec<&str> = s.split_ascii_whitespace().collect();
                        let provider = Provider{ip_addr: parts[0].to_string(), btc_addr: parts[1].to_string()};
                        let lock = db.lock().unwrap();
                        lock.push(provider);
                        drop(lock);
                        let serialized = serde_json::to_string(&lock).unwrap();
                        println!("{}", serialized);
                        //let mut f = File::open("./providers.json").await.unwrap();
                        //serde_json::to_writer_pretty(&fs::File::open("./providers.json"), value)
                    },
                Err(e) => println!("{}",e)
            };
            
        
                
        });
    }
}
