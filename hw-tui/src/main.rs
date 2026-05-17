fn main() {
    if let Err(error) = hw_tui::run_stdio() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
