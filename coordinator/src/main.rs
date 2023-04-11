use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::str;
use std::sync::{Arc, Mutex as std_mutex};
use serde_derive::{Deserialize, Serialize};
use rand::prelude::*;

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

    let listener = TcpListener::bind("localhost:8080").await.unwrap();
    let db = Arc::new(std_mutex::new(Providers{providers: Vec::new()}));

    loop{
        let (mut socket, _) = listener.accept().await.unwrap();
        let db = db.clone();

        tokio::spawn(async move{
            println!("Connection opened");
            let mut buf = [0u8; 56];
            //let (mut reader, _) = socket.split();
            
            match socket.read(&mut buf).await{
                Ok(0) => return,
                Ok(_n) =>{
                        let message = String::from_utf8_lossy(&buf);
                        let parts: Vec<&str> = message.split_ascii_whitespace().collect();
                        //ADD PROVIDER TO JSON
                        if parts[0].eq("p"){
                            let provider = Provider{
                                ip_addr: parts[1].to_string(), 
                                btc_addr: parts[2].to_string()
                            };
    
                            let mut db = db.lock().unwrap();
                            db.providers.push(provider);
                            let serialized = serde_json::to_string_pretty(&db.providers).unwrap();
                            drop(db);
                        
                            std::fs::write("./providers.json", serialized).unwrap(); 
                        }
                        //SEND PROVIDER FROM JSON
                        else{
                            let text = std::fs::read_to_string("./providers.json").unwrap();
                            let providers = serde_json::from_str::<Vec<Provider>>(&text).unwrap();
                            let n = rand::thread_rng().gen_range(0..providers.len());
                            let provider = providers.get(n).unwrap();
                            
                            let message = provider.ip_addr.to_string() + " " + &provider.btc_addr;
                            socket.write_all(message.as_bytes()).await.unwrap();
                        }
                        
                    },
                Err(e) => println!("{}",e)
            };    
        });
    }
}
