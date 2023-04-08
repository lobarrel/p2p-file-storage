use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::net::{TcpStream, TcpListener};
use std::{
    io as std_io
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
                        println!("download");
                    }
                    if let KeyCode::Char('q') = key.code {
                        return;
                    }
                }
            }
            //connect_to_server().await.unwrap();
        }
        if let KeyCode::Char('2') = key.code {
            start_server().await.unwrap();
        }
        if let KeyCode::Char('q') = key.code {
            return;
        }
    }

    // restore terminal
    execute!(terminal.backend_mut(),LeaveAlternateScreen).unwrap();
    terminal.show_cursor().unwrap();
    
}



async fn upload_file() -> io::Result<()>{
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();

    let mut f = File::open("./data.txt").await?;
    let mut buffer = Vec::new();

    // read the whole file
    f.read_to_end(&mut buffer).await?;
    println!("{:?}", buffer);

    stream.write_all(&mut buffer).await?;


    Ok(())
}

async fn start_server() -> io::Result<()>{
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
