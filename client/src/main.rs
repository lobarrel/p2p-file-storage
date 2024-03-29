use bdk::{Wallet, blockchain::ElectrumBlockchain, sled::Tree};
use chacha20poly1305::{
    aead::{stream, Aead, NewAead},
    XChaCha20Poly1305,
};
use rand::distributions::Alphanumeric;
use securestore::{SecretsManager, KeySource};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::net::tcp::{WriteHalf, ReadHalf};
use tokio::net::{TcpStream, TcpListener};
use std::{path::{Path, PathBuf}, hash};
use std::process::Command;
use std::sync::{Arc, Mutex as std_mutex, MutexGuard};
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

mod bdkwallet;
use bdkwallet::*;

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
    content: Vec<u8>
}

struct StoredFiles{
    stored_files: Vec<StoredFile>
}


const COORDINATOR_IP: &str = "localhost:8080";
const MAX_PROVIDER_ID: u16 = 65535;
const CAPACITY: u64 = 128000000;    //in bytes
const SATS_X_KB: u64 = 100;

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

            let wallet: Wallet<ElectrumBlockchain, Tree>;
            if !Path::new("./secrets.key").exists(){
                store_encryption_key();
                println!("Creating your Bitcoin wallet...");
                let (receive_desc, change_desc) = get_descriptors();
                store_descriptors(&receive_desc, &change_desc);
                wallet = new_wallet(".db-user", receive_desc, change_desc).unwrap();
                println!("{}", format!("Wallet successfully created!").green());
            }else{
                let sec_man = SecretsManager::load("secrets.json", KeySource::File("secrets.key")).unwrap();
                let receive_desc = sec_man.get("receive_desc").unwrap();
                let change_desc = sec_man.get("change_desc").unwrap();
                wallet = new_wallet(".db-user", receive_desc, change_desc).unwrap();
            }

            println!("\nCommands:\na: show your Bitcoin address\nb: show your wallet balance\nu: upload a new file\nd: download a file\nq: quit");
        
            loop{
                if let Event::Key(key) = event::read().unwrap(){
                    if let KeyCode::Char('a') = key.code {
                        println!("Address: {}", get_wallet_address(&wallet));
                    }
                    if let KeyCode::Char('b') = key.code {
                        println!("Balance: {}", get_wallet_balance(&wallet));
                    }
                    if let KeyCode::Char('u') = key.code {
                        println!("Insert the file path:");
                        let user_input: String = read!("{}\n");
                        match upload_file(user_input, &wallet).await{
                            Ok(()) => println!("{}", format!("File uploaded").green()),
                            Err(e) => println!("{}", e)
                        };
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
        }
        if let KeyCode::Char('2') = key.code {

            let wallet: Wallet<ElectrumBlockchain, Tree>;
            if !Path::new("./secrets.key").exists(){
                store_encryption_key();
                println!("Creating your Bitcoin wallet...");
                let (receive_desc, change_desc) = get_descriptors();
                store_descriptors(&receive_desc, &change_desc);
                wallet = new_wallet(".db-provider", receive_desc, change_desc).unwrap();
                println!("{}", format!("Wallet successfully created!").green());
            }else{
                let sec_man = SecretsManager::load("secrets.json", KeySource::File("secrets.key")).unwrap();
                let receive_desc = sec_man.get("receive_desc").unwrap();
                let change_desc = sec_man.get("change_desc").unwrap();
                wallet = new_wallet(".db-provider", receive_desc, change_desc).unwrap();
            }

            println!("Insert the port number for TCP connection (8080 suggested):");
            stdin.read_line(&mut user_input).unwrap();
            signup_as_provider(user_input, &wallet).await.unwrap();
            
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




async fn signup_as_provider(tcp_port: String, wallet: &Wallet<ElectrumBlockchain, Tree>) -> io::Result<()>{
    let mut stream = TcpStream::connect(COORDINATOR_IP).await.unwrap();
    let id = rand::thread_rng().gen_range(0..=MAX_PROVIDER_ID).to_string();
    let ip_addr = local_ip().unwrap().to_string() + ":" + &tcp_port;
    let btc_addr = get_wallet_address(wallet).to_string();
    let message = "p ".to_string() + &id + " " + &ip_addr + " " + &btc_addr;
    //println!("{}", message);
    println!("{}", format!("Running storage provider.\nBitcoin address: {}", btc_addr).green());

    stream.write(message.as_bytes()).await?;
    Ok(())
}




async fn ask_coordinator(socket: &mut TcpStream, provider_id: String) -> Result<Provider, ()>{
    let (mut rd, mut wr) = socket.split();

    let message = "c ".to_string() + &provider_id;
    wr.write(message.as_bytes()).await.unwrap();

    let mut buf = [0u8; 128];
            
   
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





async fn upload_file(filepath: String, wallet: &Wallet<ElectrumBlockchain, Tree>) -> Result<(), String>{
    
    let file_path = Path::new(&filepath);
    let file_name = file_path.file_name().unwrap().to_str().unwrap();
    let file_size = file_path.metadata().unwrap().len();

    println!("{}", file_size);
    let amount = file_size * SATS_X_KB;

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
        let parts: Vec<&str> = message.split_ascii_whitespace().collect();
        if !parts[0].eq("err:"){

            //SEND TRANSACTION
            println!("Sending {} sats to storage provider...", amount);
            match new_transaction(wallet, provider.btc_addr.to_string(), amount){
                Ok(()) => {
                    println!("{}", format!("Transaction completed").green());

                    //ENCRYPT AND SEND FILE
                    let encrypted_data = encrypt_file(&filepath);
                    wr.write(encrypted_data.as_slice()).await.unwrap();
                    let hash1 = digest(file_name);
                    let hash2 = digest(&*encrypted_data);
                    let file_hash = digest(hash1 + &hash2);

                    //ADD FILE INFO
                    let new_file = FileInfo{
                        hash: file_hash,
                        name: file_name.to_string(),
                        provider_id: provider.id.to_string()
                    };
                    my_files.push(new_file);

                    let serialized = serde_json::to_string_pretty(&my_files).unwrap();
                    std::fs::write("./my_files.json", serialized).unwrap(); 
                },
                Err(_e) => println!("Transaction failed")
            };
        }
    }
    Ok(())
}


async fn download_file(filename: String, directory: String){
    let filepath = directory + &filename;
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
        let mut file_content = Vec::<u8>::new();
                
        loop{
            match rd.read(&mut buf).await {
                Ok(0) => break,
                Ok(_n) => {
                    file_content.push(buf[0]);
                },
                Err(e) => println!("{}", e)
            }
        }
      
        let hash1 = digest(filename);
        let hash2 = digest(&*file_content);
        if digest(hash1 + &hash2).eq(&file.hash){    
            let mut file = File::create(filepath).await.unwrap();
            let decrypted_data = decrypt_file(file_content);
            file.write(&decrypted_data).await.unwrap();
            println!("{}", format!("Download completed").green());
        }else{
            println!("{}", format!("Could not complete download").red());
        }
    }else{
        println!("{}", format!("File does not exist").red());
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
                if parts[1].parse::<u64>().unwrap() > CAPACITY{
                    wr.write("error: file exceeds storage capacity limit".as_bytes()).await.unwrap();
                }else{
                    wr.write("uploading file...".as_bytes()).await.unwrap();
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
                    Ok(file) => wr.write_all(&file.content).await.unwrap(),
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
            
     let mut file_content = Vec::<u8>::new();
     loop {
         match rd.read(&mut buf).await{
             Ok(0) => break,
             Ok(_n) =>{
                 file_content.push(buf[0]);
                 },
             Err(e) => println!("{}",e)
         };
     }  

     let hash1 = digest(filename);
     let hash2 = digest(&*file_content);

     let new_file = StoredFile{
         hash: digest(hash1 + &hash2),
         content: file_content
     };
     return new_file;
     
}

fn encrypt_file(filepath: &str) -> Vec<u8>{
    let sec_man = SecretsManager::load("secrets.json", KeySource::File("secrets.key")).unwrap();
    let key = sec_man.get("encryption_key").unwrap();
    let nonce = sec_man.get("nonce").unwrap();
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());

    let file_data = fs::read(filepath).unwrap();
    let encrypted_data = cipher
        .encrypt(nonce.as_bytes().into(), file_data.as_ref()).unwrap();

    return encrypted_data;
}

fn decrypt_file(file_data: Vec<u8>) -> Vec<u8>{
    let sec_man = SecretsManager::load("secrets.json", KeySource::File("secrets.key")).unwrap();
    let key = sec_man.get("encryption_key").unwrap();
    let nonce = sec_man.get("nonce").unwrap();
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());

    let decrypted_data = cipher
        .decrypt(nonce.as_bytes().into(), file_data.as_ref()).unwrap();
    
    return decrypted_data;
}

fn store_encryption_key(){
    let key: String = thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let nonce: String = thread_rng().sample_iter(&Alphanumeric).take(24).map(char::from).collect();
    println!("Your encryption key is being generated. Please create a password to complete the operation");

    Command::new("ssclient")
        .arg("create").arg("secrets.json").arg("--export-key").arg("secrets.key")
        .spawn().expect("failed to generate encryption key").wait().unwrap();
    Command::new("ssclient")
        .arg("-k").arg("secrets.key").arg("set").arg("encryption_key").arg(key)
        .spawn().expect("failed to generate encryption key").wait().unwrap();
    Command::new("ssclient")
        .arg("-k").arg("secrets.key").arg("set").arg("nonce").arg(nonce)
        .spawn().expect("failed to generate encryption key").wait().unwrap();

    println!("{}", format!("Your encryption key has been saved").green());
}

fn store_descriptors(receive_desc: &str, change_desc: &str){
    Command::new("ssclient").arg("-k").arg("secrets.key").arg("set").arg("receive_desc").arg(receive_desc)
        .spawn().expect("failed to store wallet keys").wait().unwrap();
    Command::new("ssclient").arg("-k").arg("secrets.key").arg("set").arg("change_desc").arg(change_desc)
        .spawn().expect("failed to store wallet keys").wait().unwrap();
}
