use std::io::{self, Write};

pub fn prompt(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

pub fn clear() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

pub fn print_header(title: &str) {
    println!();
    println!("  ╔══════════════════════════════════════════════════════════╗");
    println!("  ║  {:<58}║", title);
    println!("  ╚══════════════════════════════════════════════════════════╝");
    println!();
}

pub fn print_section(title: &str) {
    println!();
    println!("  ── {} {}", title, "─".repeat(55usize.saturating_sub(title.len())));
}

pub fn pause() {
    prompt("\n  [Press Enter to continue...]");
}
