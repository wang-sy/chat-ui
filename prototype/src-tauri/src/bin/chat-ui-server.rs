fn main() {
    let port = std::env::var("CHAT_UI_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(chat_ui::server::PORT);
    if let Err(error) = chat_ui::server::serve(port, chat_ui::server::state_directory()) {
        eprintln!("Chat UI service: {error}");
        std::process::exit(1);
    }
}
