use rand::distributions::Alphanumeric;
use rand::rngs::OsRng;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::net::tcp::{WriteHalf, ReadHalf};
use tokio::net::{TcpStream, TcpListener};
use tui::text::Text;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex as std_mutex, MutexGuard};
use std::time::Duration;
use std::{
    io as std_io, str, fs, thread
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
use rand::{prelude::*, Error};
use serde_derive::{Deserialize, Serialize};
use sha256::digest;
#[macro_use] extern crate text_io;



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
const CAPACITY: u64 = 128000000;    //in bytes

#[tokio::main]
async fn main(){

    let mut stdout = std_io::stdout();
    let stdin = std_io::stdin();
    let mut user_input = String::new();
    execute!(stdout, EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    println!("{}", format!("P2P FILE STORAGE\n\n").bold());
    println!("Press '1' if you want to upload your files on the P2P storage service\nPress '2' if you want to provide storage space and earn commissions\nPress 'q' to quit\n");

    if let Event::Key(key) = event::read().unwrap(){
        if let KeyCode::Char('1') = key.code {
            if !Path::new("./secrets.key").exists(){
                store_encryption_key();
            }
        
            println!("Creating your Bitcoin wallet...");
            println!("{}", format!("Wallet successfully created!").green());
            println!("\nCommands:\na: show your Bitcoin address\nb: show your wallet balance\nu: upload a new file\nd: download a file\nq: quit");
        
            //println!("{}", format!("Your secret key is: {}", String::from_utf8_lossy(&key)).green());
            
            loop{
                if let Event::Key(key) = event::read().unwrap(){
                    if let KeyCode::Char('a') = key.code {
                        println!("address");
                    }
                    if let KeyCode::Char('b') = key.code {
                        println!("balance");
                    }
                    if let KeyCode::Char('u') = key.code {
                        println!("Insert the file path:");
                        let user_input: String = read!("{}\n");
                        upload_file(user_input).await.unwrap();
                    }
                    if let KeyCode::Char('d') = key.code {
                        println!("Insert the file name:");
                        let filename: String = read!("{}\n");
                        println!("Insert the directory where you want to save the file (must terminate with /):");
                        let directory: String = read!("{}\n");
                        download_file(filename, directory).await;
                    }
                    if let KeyCode::Char('q') = key.code {
                        return;
                    }
                }
            }
            //connect_to_server().await.unwrap();
        }
        if let KeyCode::Char('2') = key.code {
            println!("Insert the port number for TCP connection (8080 suggested):");
            stdin.read_line(&mut user_input).unwrap();
            signup_as_provider(user_input).await.unwrap();
            
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




async fn signup_as_provider(tcp_port: String) -> io::Result<()>{
    let mut stream = TcpStream::connect(COORDINATOR_IP).await.unwrap();
    let id = rand::thread_rng().gen_range(0..=MAX_PROVIDER_ID).to_string();
    let ip_addr = local_ip().unwrap().to_string() + ":" + &tcp_port;
    let btc_addr = "tb1qkkgjylluap72wnhz6rf5adxvhpd3wa6u6e0coc".to_string();
    let message = "p ".to_string() + &id + " " + &ip_addr + " " + &btc_addr;
    //println!("{}", message);

    stream.write(message.as_bytes()).await?;
    Ok(())
}




async fn ask_coordinator(socket: &mut TcpStream, provider_id: String) -> Result<Provider, ()>{
    let (mut rd, mut wr) = socket.split();

    let message = "c ".to_string() + &provider_id;
    wr.write(message.as_bytes()).await.unwrap();

    let mut buf = [0u8; 64];
            
   
    let n = rd.read(&mut buf).await.unwrap();
    if n == 0 {
        println!("Errore in lettura");
    }
    let message = str::from_utf8(&buf[..n]).unwrap();
    
    let parts: Vec<&str> = message.split_ascii_whitespace().collect();
    let provider = Provider{
        id: parts[0].to_string(),
        ip_addr: parts[1].to_string(),
        btc_addr: parts[2].to_string()
    };
    
    Ok(provider)
}  





async fn upload_file(file_path: String) -> Result<(), String>{
    let file_path = Path::new(&file_path);
    let file_name = file_path.file_name().unwrap().to_str().unwrap();
    let file_size = file_path.metadata().unwrap().len();


    let serialized = std::fs::read_to_string("./my_files.json").unwrap();
    let mut my_files = Vec::<FileInfo>::new();
    if !serialized.is_empty(){
        my_files = serde_json::from_str::<Vec<FileInfo>>(&serialized).unwrap();
    }
    if my_files.iter().any(|elem| elem.name.eq(file_name)){
        return Err("File with this name already exists. Change file name".to_string());
    }

    
    
 
    let mut socket = TcpStream::connect(COORDINATOR_IP).await.unwrap();
    let provider = ask_coordinator(&mut socket, "n".to_string()).await.unwrap();
    //println!("RESULT: {} {} {}", provider.id, provider.ip_addr, provider.btc_addr);


    let ip_prov = provider.ip_addr;
    let mut socket = TcpStream::connect(&ip_prov).await.unwrap();
    let (mut rd, mut wr) = socket.split();


    let message = "u ".to_string() + &file_size.to_string() + " " + &file_name;
    wr.write(message.as_bytes()).await.unwrap();

    let mut buf = [0u8; 64];
    let n = rd.read(&mut buf).await.unwrap();
    if n == 0{
        println!("Errore in lettura");
    }
    else{
        let message = str::from_utf8(&buf[..n]).unwrap();
        println!("{}", message);
    }
   
    
    // //TODO: encrypt file
    let mut f = File::open(&file_path).await.unwrap();
    let mut file_buf = Vec::new();
    let n = f.read_to_end(&mut file_buf).await.unwrap();
    
    let file_hash = digest(file_name.to_string() + &String::from_utf8_lossy(&file_buf[..n]));

    wr.write_all(&mut file_buf).await.unwrap();
 
    let new_file = FileInfo{
        hash: file_hash,
        name: file_name.to_string(),
        provider_id: provider.id.to_string()
    };
    my_files.push(new_file);
    
    let serialized = serde_json::to_string_pretty(&my_files).unwrap();
    std::fs::write("./my_files.json", serialized).unwrap(); 

    Ok(())
}


async fn download_file(filename: String, directory: String){
    let text = std::fs::read_to_string("./my_files.json").unwrap();
    let mut my_files = Vec::<FileInfo>::new();
    if !text.is_empty(){
        my_files = serde_json::from_str::<Vec<FileInfo>>(&text).unwrap();
    }

    if my_files.iter().any(|elem| elem.name.eq(&filename)){
        let n = my_files.iter().position(|elem| elem.name.eq(&filename)).unwrap();
        let file = my_files.remove(n);
        let serialized = serde_json::to_string_pretty(&my_files).unwrap();
        std::fs::write("./my_files.json", serialized).unwrap(); 
        
        let mut socket = TcpStream::connect(COORDINATOR_IP).await.unwrap();
        let provider = ask_coordinator(&mut socket, file.provider_id).await.unwrap();
        //println!("RESULT: {} {} {}", provider.id, provider.ip_addr, provider.btc_addr);

        let mut socket = TcpStream::connect(&provider.ip_addr).await.unwrap();
        let (mut rd, mut wr) = socket.split();

        let message = "d ".to_string() + &file.hash;
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
      
        if digest(format!("{}{}", filename, file_content)).eq(&file.hash){
            let filepath = directory + &filename;
            let mut file = File::create(filepath).await.unwrap();
            file.write(file_content.as_bytes()).await.unwrap();
            println!("download completed");
        }else{
            println!("could not complete download");
        }
    }else{
        println!("File does not exist");
    }
}



async fn run_provider() -> io::Result<()>{
    let ip_addr = local_ip().unwrap().to_string() + ":8080";
    let listener = TcpListener::bind(ip_addr).await.unwrap();
    let db = Arc::new(std_mutex::new(StoredFiles{stored_files: Vec::new()}));

    loop{
        let (mut socket, _) = listener.accept().await.unwrap();
        let db: Arc<std_mutex<StoredFiles>> = db.clone();
        
        tokio::spawn(async move{
            println!("Connection opened");
            let (mut rd, mut wr) = socket.split();

            let mut buf = [0u8; 128];
            let n = rd.read(&mut buf).await.unwrap();
            if n == 0 {
                println!("Errore in lettura");
            }
            let bytes = &buf[..n];
            let message = str::from_utf8(bytes).unwrap();
            let parts: Vec<&str> = message.split_ascii_whitespace().collect();

            //UPLOAD
            if parts[0].eq("u"){
                //TODO check size
                if parts[1].parse::<u64>().unwrap() > CAPACITY{
                    wr.write("file exceeds storage capacity limit".as_bytes()).await.unwrap();
                }else{
                    wr.write("file uploaded".as_bytes()).await.unwrap();
                    let uploaded_file = read_uploaded_file(rd, parts[2].to_string()).await;

                    let mut db = db.lock().unwrap();
                    db.stored_files.push(uploaded_file);

                    let serialized = serde_json::to_string_pretty(&db.stored_files).unwrap();
                    std::fs::write("./stored_files.json", serialized).unwrap(); 
                    
                }
            }

            //DOWNLOAD
            else {
                let hash = parts[1].to_string();
                println!("{}",hash);
                
                let downloaded_file: Result<StoredFile, String>;
                {
                    let mut db = db.lock().unwrap();
                    downloaded_file = if db.stored_files.iter().any(|elem| elem.hash.eq(&hash)){
                        let n = db.stored_files.iter().position(|elem| elem.hash.eq(&hash)).unwrap();
                        Ok(db.stored_files.remove(n))
         
                    }else{
                        Err("hash not found".to_string())
                    };
    
                    let serialized = serde_json::to_string_pretty(&db.stored_files).unwrap();
                    std::fs::write("./stored_files.json", serialized).unwrap(); 
                }

                match downloaded_file{
                    Ok(file) => wr.write_all(&mut file.content.as_bytes()).await.unwrap(),
                    Err(e) => {
                        wr.write(e.as_bytes()).await.unwrap();
                    }
                }
                
            }
        });
    }
    
}


async fn read_uploaded_file(mut rd: ReadHalf<'_>, filename: String) -> StoredFile{
     let mut buf = [0u8; 1];
            
     let mut file_content = "".to_string();
     loop {
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
         hash: digest(filename + file_content.as_str()),
         content: file_content
     };
     return new_file;
     
}


fn store_encryption_key(){
    let key: String = thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut nonce);
    println!("Your encryption key is being generated. Please create a password to complete the operation");

    Command::new("ssclient")
        .arg("create").arg("secrets.json").arg("--export-key").arg("secrets.key")
        .spawn().expect("failed to generate encryption key").wait().unwrap();
    Command::new("ssclient")
        .arg("-k").arg("secrets.key").arg("set").arg("encryption_key").arg(key)
        .spawn().expect("failed to generate encryption key").wait().unwrap();

    println!("{}", format!("Your encryption key has been saved").green());
}
/*

signup_as_provider:  client(P)    p [id][ip][btc]       coordinator -> save provider.json

ask_coordinator:     client(U)    c [id]                coordinator -> return Provider

upload_file:         client(U)    u [size][file]              client(P) -> save stored_files.json

download_file:       client(U)    d [hash]              client(P) -> return file  

 */