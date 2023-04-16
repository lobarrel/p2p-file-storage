use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::WriteHalf;
use tokio::net::{TcpListener};
use std::str;
use std::sync::{Arc, Mutex as std_mutex, MutexGuard};
use serde_derive::{Deserialize, Serialize};
use rand::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
struct Provider {
    id: String,
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
            
            let (mut rd, wr) = socket.split();
            
            let mut buf = [0u8; 64];

   
            let n = rd.read(&mut buf).await.unwrap();
            if n == 0 {
                println!("Errore in lettura");
            }
            let bytes = &buf[..n];
            let message = str::from_utf8(bytes).unwrap();
            let parts: Vec<&str> = message.split_ascii_whitespace().collect();

            //ADD PROVIDER TO JSON
            if parts[0].eq("p"){
                let db = db.lock().unwrap();
                add_provider(parts, db);
            }
            //SEND PROVIDER TO CLIENT
            else{
                send_provider_to_client(wr).await;
            }

            
        });
    }
}



fn add_provider(parts: Vec<&str>, mut db: MutexGuard<Providers>){
    let provider = Provider{
        id: parts[1].to_string(),
        ip_addr: parts[2].to_string(), 
        btc_addr: parts[3].to_string()
    };

    db.providers.push(provider);
    let serialized = serde_json::to_string_pretty(&db.providers).unwrap();
    
    std::fs::write("./providers.json", serialized).unwrap(); 
}


async fn send_provider_to_client(mut socket: WriteHalf<'_>){
    let text = std::fs::read_to_string("./providers.json").unwrap();
    let providers = serde_json::from_str::<Vec<Provider>>(&text).unwrap();
    let n = rand::thread_rng().gen_range(0..providers.len());
    let provider = providers.get(n).unwrap();
    
    let message = provider.id.to_string() + " " + &provider.ip_addr + " " + &provider.btc_addr;
    println!("{}",message);
    socket.write_all(message.as_bytes()).await.unwrap();
}