use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead};
use std::thread;
use std::sync::{Arc, Mutex};

struct AppState {
    current_state: Mutex<String>,
    current_scores: Mutex<String>,
}

fn handle_client(stream: TcpStream, app_state: Arc<AppState>) {
    let mut reader = BufReader::new(stream);
    let mut keep_socket_open = true;
    while keep_socket_open {
        let mut buffer = String::new();
        // Read until a newline is detected
        match reader.read_line(&mut buffer) {
            Ok(_) => {
                let received_state = buffer.trim_end().to_string();
                if received_state.len() == 0 {
                    keep_socket_open = false;
                    continue;
                }
                match app_state.current_state.lock() {
                    Ok(mut current_state) => {
                        // update current_state
                        if !received_state.clone().eq_ignore_ascii_case("{\"type\":\"dummy\"}") {
//                            println!("Received: {}", received_state.clone());
                            println!("State Update received");
                            *current_state = received_state;
                        }
                    },
                    Err(_) => {
                        eprintln!("Failed to lock current_state");
                    }
                }
            },
            Err(e) => {
                eprintln!("Failed to read from connection: {}", e);
                keep_socket_open = false;
            }
        }
    }
}

async fn get_state(app_state: web::Data<Arc<AppState>>) -> impl Responder {
    //println!("Received getState request");
    match app_state.current_state.lock() {
        Ok(current_state) => {
            HttpResponse::Ok().body(current_state.to_string())
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

async fn get_scores(app_state: web::Data<Arc<AppState>>) -> impl Responder {
    //println!("Received getScores request");
    match app_state.current_scores.lock() {
        Ok(current_scores) => {
            HttpResponse::Ok().body(current_scores.to_string())
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

#[post("/updateScores")]
async fn update_scores(body: String, app_state: web::Data<Arc<AppState>>, ) -> impl Responder {
    println!("Received updateScores request");
    match app_state.current_scores.lock() {
        Ok(mut current_scores) => {
            *current_scores = body.clone();
            //println!("Updated scores: {}", body);
            HttpResponse::Ok().body("".to_string())
        },
        Err(_) => {
            return HttpResponse::InternalServerError().body("Error");
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = Arc::new(AppState {
        current_state: Mutex::new("".to_string()),
        current_scores: Mutex::new("".to_string()),
    });

    // Bind the TCP listener to a local address
    let listener = TcpListener::bind("0.0.0.0:8086")?;
    println!("TCP Server listening on port 8086");
    let cloned_app_state = Arc::clone(&app_state);
    // Wrap the listener in an Arc to share it between threads
    let listener = Arc::new(listener);
    thread::spawn(move || {
        // Accept incoming connections in a loop
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let _listener = Arc::clone(&listener);
                    let state = Arc::clone(&cloned_app_state);
                    thread::spawn(move || {
                        // Handle the connection
                        handle_client(stream, state);
                    });
                },
                Err(e) => {
                    eprintln!("Failed to accept a connection: {}", e);
                }
            }
        }
    });

    println!("Starting HTTP server on port 8085");
    // Start the web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .route("/getState", web::get().to(get_state))
            .route("/getScores", web::get().to(get_scores))
            .service(update_scores)
    })
    .bind("0.0.0.0:8085")?
    .run()
    .await
}
