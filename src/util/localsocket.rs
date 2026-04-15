use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};
use std::io::{Write, BufWriter, BufRead, BufReader};
use std::sync::mpsc;

fn get_socket_name() -> String {
    match NameTypeSupport::query() {
        // Unix 
        NameTypeSupport::OnlyPaths => {
            let exe_path = std::env::current_exe().unwrap();
            let sock_file = format!("{}egscribe_app.sock", exe_path.parent().map(|p| p.to_path_buf()).unwrap().display());
            //"/tmp/egscribe_app.sock".to_string()
            sock_file
        }
        // Windows 
        NameTypeSupport::OnlyNamespaced | NameTypeSupport::Both => "@egscribe_app.sock".to_string(), 
    }
}

fn send_message_to_server(message: &str) -> std::io::Result<()> {
    let socket_name = get_socket_name();
    let mut conn = LocalSocketStream::connect(socket_name)?;
    let mut writer = BufWriter::new(&mut conn);
    writeln!(writer, "{}", message)?;
    writer.flush()?;
    
    Ok(())
}

fn setup_ipc_listener() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    
    std::thread::spawn(move || {
        let socket_name = get_socket_name();
        
        // remove old socket file
        if socket_name.starts_with('/') {
            let _ = std::fs::remove_file(&socket_name);
        }
        
        let listener = match LocalSocketListener::bind(socket_name.clone()) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind to socket: {}", e);
                return;
            }
        };
        
        println!("ipc server listening on {}", socket_name);
        
        for conn in listener.incoming() {
            match conn {
                Ok(conn) => {
                    let mut reader = BufReader::new(conn);
                    let mut buffer = String::new();
                    if reader.read_line(&mut buffer).is_ok() {
                        let _ = tx.send(buffer.trim().to_string());
                    }
                }
                Err(e) => eprintln!("Connection error: {}", e),
            }
        }
    });
    
    rx
}

pub fn start_process(need_open_file: &str) -> Option<mpsc::Receiver<String>> {
    let echo = send_message_to_server(need_open_file);
    if echo.is_err() {
        Some(setup_ipc_listener())
    }  else {
        None
    }
}
