mod cli;
mod error;

fn main() {
    if let Err(err) = cli::execute() {
        eprintln!("error: {}", err.to_string());
        std::process::exit(1);
    }
}
