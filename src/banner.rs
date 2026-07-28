use colored::Colorize;

/// M-Pesa's brand green, used for the banner and other one-off accents.
/// Everyday status output still uses `colored`'s named colors (green/red/
/// yellow) so it degrades sensibly on terminals without truecolor support.
const BRAND_GREEN: (u8, u8, u8) = (0, 166, 81);

const MARK: &str = r"███╗   ███╗
████╗ ████║
██╔████╔██║
██║╚██╔╝██║
██║ ╚═╝ ██║
╚═╝     ╚═╝";

/// Prints the full startup banner: the "M" mark plus tagline. Shown once,
/// only when `mpesa-dev` is run with no subcommand.
pub fn print_full() {
    println!();
    for line in MARK.lines() {
        println!(
            "{}",
            line.truecolor(BRAND_GREEN.0, BRAND_GREEN.1, BRAND_GREEN.2)
        );
    }
    println!();
    println!("{}", "M-Pesa Developer Toolkit".bold());
    println!();
}

/// Prints a short, consistent header before a command's own output —
/// e.g. `header("Inspector")` -> "Starting Inspector...". Used by every
/// subcommand so the tool feels like one product instead of four scripts.
pub fn header(title: &str) {
    println!("{}", format!("Starting {title}...").bold());
    println!();
}

/// Shared icon language used everywhere status is reported, so a reader
/// only has to learn one visual vocabulary across doctor/inspect/tunnel/
/// replay: ✓ good, ✗ bad, ⚠ caution, ○ skipped/inactive, → an action or
/// pointer to something the user should do next.
pub mod icon {
    use colored::{ColoredString, Colorize};

    pub fn ok() -> ColoredString {
        "✓".green().bold()
    }

    pub fn fail() -> ColoredString {
        "✗".red().bold()
    }

    pub fn warn() -> ColoredString {
        "⚠".yellow().bold()
    }

    pub fn skip() -> ColoredString {
        "○".dimmed()
    }

    pub fn arrow() -> ColoredString {
        "→".cyan().bold()
    }
}
