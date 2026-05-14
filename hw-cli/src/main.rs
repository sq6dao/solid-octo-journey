fn main() {
    if let Err(error) = hw_cli::session::run_stdio() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
