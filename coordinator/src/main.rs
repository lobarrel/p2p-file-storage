use tokio::io::{self, AsyncReadExt};
use tokio::net::{TcpListener};
use std::str;
use std::sync::{Arc, Mutex as std_mutex};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct Provider {
    ip_addr: String,
    btc_addr: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Providers{
    providers: Vec<Provider>
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
    let db = Arc::new(std_mutex::new(Providers{providers: Vec::new()}));
    
    //let mut f = File::create("./providers.json").await.unwrap();
    

    loop{
        let (mut socket, _) = listener.accept().await.unwrap();
        let db = db.clone();
        //let mut f = File::open("./providers.json").await.unwrap();
        

        tokio::spawn(async move{
            println!("Connection opened");
           
            let mut buf = [0u8; 54];

            let (mut reader, _) = socket.split();
            
            
            match reader.read(&mut buf).await{
                Ok(0) => return,
                Ok(_n) =>{
                        let message = String::from_utf8_lossy(&buf);
                        println!("{}", message);
                        let parts: Vec<&str> = message.split_ascii_whitespace().collect();
                        let provider = Provider{ip_addr: parts[0].to_string(), btc_addr: parts[1].to_string()};

                        let serialized: String;
                        {
                            let mut db = db.lock().unwrap();
                            db.providers.push(provider);
                            serialized = serde_json::to_string_pretty(&db.providers).unwrap();
                            drop(db);
                            println!("{}", serialized);
                        }
                        
                        std::fs::write("./providers.json", serialized).unwrap();
                        
                    },
                Err(e) => println!("{}",e)
            };
            
        
                
        });
    }
}
