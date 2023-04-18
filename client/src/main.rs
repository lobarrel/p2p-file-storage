use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::net::{TcpStream, TcpListener};
use std::path::Path;
use std::sync::{Arc, Mutex as std_mutex, MutexGuard};
use std::{
    io as std_io, str
};
use tui::{
    backend::{CrosstermBackend},
    Terminal
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    style::Stylize,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use local_ip_address::local_ip;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

struct Provider{
    id: String,
    ip_addr: String,
    btc_addr: String
}

#[derive(Deserialize, Serialize, Debug)]
struct FileInfo{
    hash: String,
    name: String,
    provider_id: String
}
#[derive(Deserialize, Serialize, Debug)]
struct StoredFile{
    hash: String,
    content: String
}

struct StoredFiles{
    stored_files: Vec<StoredFile>
}


const COORDINATOR_IP: &str = "localhost:8080";
const MAX_PROVIDER_ID: u16 = 65535;

#[tokio::main]
async fn main(){

    let mut stdout = std_io::stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    println!("{}", format!("P2P FILE STORAGE\n\n").bold());
    println!("Press '1' if you want to upload your files on the P2P storage service\nPress '2' if you want to provide storage space and earn commissions\nPress 'q' to quit\n");

    if let Event::Key(key) = event::read().unwrap(){
        if let KeyCode::Char('1') = key.code {
            println!("Creating your Bitcoin wallet...");
            println!("{}", format!("Wallet successfully created!").green());
            println!("\nCommands:\na: show your Bitcoin address\nb: show your wallet balance\nu: upload a new file\nd: download a file\nq: quit");
            loop{
                if let Event::Key(key) = event::read().unwrap(){
                    if let KeyCode::Char('a') = key.code {
                        println!("address");
                    }
                    if let KeyCode::Char('b') = key.code {
                        println!("balance");
                    }
                    if let KeyCode::Char('u') = key.code {
                        upload_file().await.unwrap();
                    }
                    if let KeyCode::Char('d') = key.code {
                        download_file("file.txt".to_string()).await;
                    }
                    if let KeyCode::Char('q') = key.code {
                        return;
                    }
                }
            }
            //connect_to_server().await.unwrap();
        }
        if let KeyCode::Char('2') = key.code {
            signup_as_provider().await.unwrap();
            
            run_provider().await.unwrap();
        }
        if let KeyCode::Char('q') = key.code {
            return;
        }
    }

    // restore terminal
    //execute!(terminal.backend_mut(),LeaveAlternateScreen).unwrap();
    //terminal.show_cursor().unwrap();
    
}




async fn signup_as_provider() -> io::Result<()>{
    let mut stream = TcpStream::connect(COORDINATOR_IP).await.unwrap();
    let id = rand::thread_rng().gen_range(0..=MAX_PROVIDER_ID).to_string();
    let ip_addr = local_ip().unwrap().to_string();
    let btc_addr = "tb1qkkgjylluap72wnhz6rf5adxvhpd3wa6u6e0coc".to_string();
    let message = "p ".to_string() + &id + " " + &ip_addr + " " + &btc_addr;
    println!("{}", message);

    stream.write(message.as_bytes()).await?;
    Ok(())
}




async fn ask_coordinator(socket: &mut TcpStream, provider_id: String) -> Result<Provider, ()>{
    //let mut socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let (mut rd, mut wr) = socket.split();

    let message = "c ".to_string() + &provider_id;
    wr.write(message.as_bytes()).await.unwrap();

    let mut buf = [0u8; 64];
            
   
    let n = rd.read(&mut buf).await.unwrap();
    if n == 0 {
        println!("Errore in lettura");
    }
    let bytes = &buf[..n];
    let message = str::from_utf8(bytes).unwrap();
    
    let parts: Vec<&str> = message.split_ascii_whitespace().collect();
    let provider = Provider{
        id: parts[0].to_string(),
        ip_addr: parts[1].to_string(),
        btc_addr: parts[2].to_string()
    };
    
    Ok(provider)
}  





async fn upload_file() -> io::Result<()>{
    let mut socket = TcpStream::connect(COORDINATOR_IP).await.unwrap();
    let provider = ask_coordinator(&mut socket, "n".to_string()).await.unwrap();
    println!("RESULT: {} {} {}", provider.id, provider.ip_addr, provider.btc_addr);


    let ip_prov = provider.ip_addr + ":8080";
    let mut socket = TcpStream::connect(&ip_prov).await.unwrap();
    let (mut rd, mut wr) = socket.split();

    let path = Path::new("./file.txt");
    let filename = path.file_name().unwrap().to_str().unwrap();
    //TODO: check name not already existing
    
    let mut f = File::open(path).await?;
    let mut buffer = Vec::new();
    // let file_size = fs::metadata(path).unwrap().len();
    // //TODO: encrypt file
    

    // let message = "u ".to_string() + &file_size.to_string();
    // wr.write(message.as_bytes()).await.unwrap();

    // let result = match rd.read(&mut buffer).await{
    //     Ok(0) => Err(()),
    //     Ok(_n) =>{
    //         f.read_to_end(&mut buffer).await?;
    //         wr.write_all(&mut buffer).await?;
    //         Ok(())
    //     },
    //     Err(e) => Err(println!("{}", e))
    // };

    f.read_to_end(&mut buffer).await?;
    wr.write_all(&mut buffer).await?;

    
    let text = std::fs::read_to_string("./my_files.json").unwrap();
    
    let mut my_files = Vec::<FileInfo>::new();
    if !text.is_empty(){
        my_files = serde_json::from_str::<Vec<FileInfo>>(&text).unwrap();
    }
 
    let new_file = FileInfo{
        hash: "hash".to_string(),
        name: filename.to_string(),
        provider_id: provider.id.to_string()
    };
    my_files.push(new_file);
    println!("{:?}", my_files);
    let serialized = serde_json::to_string_pretty(&my_files).unwrap();
    std::fs::write("./my_files.json", serialized).unwrap(); 

    Ok(())
}


async fn download_file(filename: String){
    let text = std::fs::read_to_string("./my_files.json").unwrap();
    let mut my_files = Vec::<FileInfo>::new();
    if !text.is_empty(){
        my_files = serde_json::from_str::<Vec<FileInfo>>(&text).unwrap();
    }

    if my_files.iter().any(|elem| elem.name.eq(&filename)){
        let n = my_files.iter().position(|elem| elem.name.eq(&filename)).unwrap();
        let provider_id = my_files.get(n).unwrap().provider_id.as_str();
        let mut socket = TcpStream::connect(COORDINATOR_IP).await.unwrap();
        let provider = ask_coordinator(&mut socket, provider_id.to_string()).await.unwrap();
        println!("RESULT: {} {} {}", provider.id, provider.ip_addr, provider.btc_addr);

        let ip_prov = provider.ip_addr + ":8080";
        let mut socket = TcpStream::connect(&ip_prov).await.unwrap();
        let (mut rd, mut wr) = socket.split();

        let message = "d ".to_string() + " " + "hash";
        wr.write(message.as_bytes()).await.unwrap();

        let mut buf = [0u8; 1];
        let mut file_content = "".to_string();
                
        loop{
            match rd.read(&mut buf).await {
                Ok(0) => break,
                Ok(_n) => {
                    let text = String::from_utf8(buf.to_vec()).unwrap();
                    file_content.push_str(&text);
                },
                Err(e) => println!("{}", e)
            }
        }
    
        let filepath = "/Users/lorenzobottelli/Desktop/".to_string() + &filename;
        let mut file = File::create(filepath).await.unwrap();
        file.write(file_content.as_bytes()).await.unwrap();
    }
}



async fn run_provider() -> io::Result<()>{
    let ip_addr = local_ip().unwrap().to_string() + ":8080";
    let listener = TcpListener::bind(ip_addr).await.unwrap();
    let db = Arc::new(std_mutex::new(StoredFiles{stored_files: Vec::new()}));

    loop{
        let (mut socket, _) = listener.accept().await.unwrap();
        let db = db.clone();
        
        tokio::spawn(async move{
            println!("Connection opened");
            let (mut rd, mut wr) = socket.split();

            let mut buf = [0u8; 64];
            let n = rd.read(&mut buf).await.unwrap();
            if n == 0 {
                println!("Errore in lettura");
            }
            let bytes = &buf[..n];
            let message = str::from_utf8(bytes).unwrap();
            let parts: Vec<&str> = message.split_ascii_whitespace().collect();

            if parts[0].eq("u"){

                //TODO: check size
                let mut buf = [0u8; 1];
            
                let mut file_content = "".to_string();
                loop {
                    //let mut file_content = file_content.clone();
                    match rd.read(&mut buf).await{
                        Ok(0) => break,
                        Ok(_n) =>{
                            let text = String::from_utf8(buf.to_vec()).unwrap();
                            file_content.push_str(&text);
                            },
                        Err(e) => println!("{}",e)
                    };
                }  
    
                let new_file = StoredFile{
                    hash: "hash".to_string(),
                    content: file_content
                };
    
                let mut db = db.lock().unwrap();
                db.stored_files.push(new_file);
    
                let serialized = serde_json::to_string_pretty(&db.stored_files).unwrap();
                std::fs::write("./stored_files.json", serialized).unwrap(); 
            }

            else {
                let hash = parts[1].to_string();
                let serialized = std::fs::read_to_string("./stored_files.json").unwrap();
                let mut stored_files = Vec::<StoredFile>::new();
                if !serialized.is_empty(){
                    stored_files = serde_json::from_str::<Vec<StoredFile>>(&serialized).unwrap();
                }

                if stored_files.iter().any(|elem| elem.hash.eq(&hash)){
                    let n = stored_files.iter().position(|elem| elem.hash.eq(&hash)).unwrap();
                    let file = stored_files.remove(n);

                    wr.write_all(&mut file.content.as_bytes()).await.unwrap();
                }

            }
        });
    }
}



/*

signup_as_provider:  client(P)    p [id][ip][btc]       coordinator -> save provider.json

ask_coordinator:     client(U)    c [id]                coordinator -> return Provider

upload_file:         client(U)    u [size][file]              client(P) -> save stored_files.json

download_file:       client(U)    d [hash]              client(P) -> return file  

 */