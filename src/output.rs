use colored::Colorize;

/// Print a pass/fail check row to stdout.
pub fn print_check(label: &str, passed: bool, detail: &str) {
    let status = if passed {
        "  PASS".green().bold()
    } else {
        "  FAIL".red().bold()
    };
    println!("{} {}", status, label);
    if !detail.is_empty() {
        println!("       {}", detail.dimmed());
    }
}

/// Print a section header.
pub fn print_header(title: &str) {
    println!("\n{}", title.bold().underline());
}

/// Print a hint on how to fix something.
pub fn print_hint(msg: &str) {
    println!("  {} {}", "→".yellow(), msg);
}
