use std::io::{self, Write};

// ── ANSI colour helpers ───────────────────────────────────────────────────────

pub const R:  &str = "\x1B[0m";   // reset
pub const DIM: &str = "\x1B[2m";  // dim
pub const BOLD: &str = "\x1B[1m"; // bold

pub const CYAN:    &str = "\x1B[36m";
pub const BCYAN:   &str = "\x1B[96m";  // bright cyan
pub const BYELLOW: &str = "\x1B[93m";  // bright yellow
pub const BGREEN:  &str = "\x1B[92m";  // bright green
pub const BRED:    &str = "\x1B[91m";  // bright red
pub const BMAGENTA:&str = "\x1B[95m";  // bright magenta
pub const BWHITE:  &str = "\x1B[97m";  // bright white

// ── Basic I/O ─────────────────────────────────────────────────────────────────

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
    println!("  {CYAN}╔══════════════════════════════════════════════════════════╗{R}");
    println!("  {CYAN}║{R}  {BWHITE}{:<58}{R}{CYAN}║{R}", title);
    println!("  {CYAN}╚══════════════════════════════════════════════════════════╝{R}");
    println!();
}

pub fn print_section(title: &str) {
    let dashes = "─".repeat(55usize.saturating_sub(title.len()));
    println!();
    println!("  {CYAN}──{R} {BOLD}{title}{R} {DIM}{dashes}{R}");
}

pub fn pause() {
    prompt(&format!("\n  {DIM}[Press Enter to continue...]{R}"));
}
