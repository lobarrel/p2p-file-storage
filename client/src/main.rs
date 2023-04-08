use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::net::{TcpStream};
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
            connect_to_server().await.unwrap();
        }
        if let KeyCode::Char('2') = key.code {
            println!("Server");
        }
        if let KeyCode::Char('q') = key.code {
            return;
        }
    }

    // restore terminal
    execute!(terminal.backend_mut(),LeaveAlternateScreen).unwrap();
    terminal.show_cursor().unwrap();
    
}



async fn connect_to_server() -> io::Result<()>{
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();

    let mut f = File::open("./data.txt").await?;
    let mut buffer = Vec::new();

    // read the whole file
    f.read_to_end(&mut buffer).await?;
    println!("{:?}", buffer);

    stream.write_all(&mut buffer).await?;


    Ok(())
}
