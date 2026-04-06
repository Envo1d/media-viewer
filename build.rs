use std::env;

fn main() {
    if env::var("CARGO_CFG_WINDOWS").is_ok() {
        let mut res = winres::WindowsResource::new();

        res.set_icon("assets/icons/icon.ico");

        if let Err(e) = res.compile() {
            eprintln!("Error compiling Windows resources: {}", e);
        }
    }
}
