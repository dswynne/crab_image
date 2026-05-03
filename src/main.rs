// Standard
use std::env;

// Local
mod equalize;
mod util;
mod gui;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Run pipeline from YAML file
        let filepath = &args[1];
        match gui::run_pipeline_cli(filepath) {
            Ok(result) => {
                println!("Pipeline executed successfully:");
                println!("{}", result);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let url = "http://127.0.0.1:3000";
        println!("Starting web GUI at {url}");
        if webbrowser::open(url).is_err() {
            println!("Open {url} in your browser if it does not open automatically.");
        }

        if let Err(err) = gui::start_server().await {
            eprintln!("Failed to start server: {err}");
            std::process::exit(1);
        }
    }
}


