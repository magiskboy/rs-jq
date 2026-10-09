mod cli;
mod error;

fn main() {
    if let Err(err) = cli::execute() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
