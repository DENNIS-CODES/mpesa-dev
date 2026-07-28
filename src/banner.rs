use colored::Colorize;

const MARK: &str = r"███╗   ███╗
████╗ ████║
██╔████╔██║
██║╚██╔╝██║
██║ ╚═╝ ██║
╚═╝     ╚═╝";

/// Ambient decoration flanking the mark — a scattering of sparks, like
/// loose change, rather than a literal scene. Purely cosmetic; alignment
/// doesn't need to be pixel-perfect.
const SPARKLE_TOP: &str = "   ✦          ·                  ✦";
const SPARKLE_BOTTOM: &str = "        ·            ✦       ·";

/// Gradient endpoints for the mark: a deep M-Pesa green at the top fading
/// to a brighter green at the bottom, so the banner reads as one
/// deliberately designed piece rather than a flat block of color. Chosen
/// so every interpolated stop still maps to a sensible plain "green" on
/// terminals without truecolor support (no COLORTERM=truecolor) — see
/// `colored`'s 16-color fallback — rather than drifting into gray or cyan.
const GRADIENT_START: (u8, u8, u8) = (0, 104, 55);
const GRADIENT_END: (u8, u8, u8) = (20, 175, 95);

const RULE_WIDTH: usize = 56;

fn lerp(start: u8, end: u8, t: f32) -> u8 {
    (start as f32 + (end as f32 - start as f32) * t).round() as u8
}

fn gradient_color(t: f32) -> (u8, u8, u8) {
    (
        lerp(GRADIENT_START.0, GRADIENT_END.0, t),
        lerp(GRADIENT_START.1, GRADIENT_END.1, t),
        lerp(GRADIENT_START.2, GRADIENT_END.2, t),
    )
}

/// Prints the full startup banner: a welcome line, the "M" mark rendered
/// as a top-to-bottom color gradient with a sparkle of ambient color
/// around it, the tagline + version + environment, and a quick command
/// guide — so the one moment a user sees this, it also teaches them
/// what's here. Shown once, only when `mpesa-dev` is run with no
/// subcommand.
pub fn print_full(environment: &str) {
    let rule = "·".repeat(RULE_WIDTH).dimmed();

    println!("{rule}");
    println!();
    println!("{}", "Welcome to mpesa-dev.".bold());
    println!();
    println!("{}", SPARKLE_TOP.yellow());

    let lines: Vec<&str> = MARK.lines().collect();
    let last_index = lines.len().saturating_sub(1).max(1) as f32;
    for (i, line) in lines.iter().enumerate() {
        let (r, g, b) = gradient_color(i as f32 / last_index);
        println!("{}", line.truecolor(r, g, b));
    }
    println!("{}", SPARKLE_BOTTOM.yellow());

    println!();
    println!(
        "{} {}",
        "M-Pesa Developer Toolkit".bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );

    let env_line = format!("Environment: {environment}");
    if environment == "production" {
        // Production talks to real money — make it impossible to miss.
        println!("{}", env_line.black().on_yellow().bold());
    } else {
        println!("{}", env_line.dimmed());
    }
    println!();

    print_command_guide();

    println!();
    println!("{rule}");
    println!();
}

fn print_command_guide() {
    let entries: [(&str, &str, Option<&str>); 4] = [
        ("doctor", "diagnose your Daraja setup", None),
        ("inspect", "watch callbacks live", Some("starting now")),
        ("tunnel", "get a public HTTPS URL, no ngrok", None),
        ("replay", "resend a captured callback", None),
    ];

    for (name, description, note) in entries {
        let padded = format!("{name:<8}");
        let note = note
            .map(|n| format!("  {}", format!("← {n}").green()))
            .unwrap_or_default();
        println!("  {}{}{}", padded.cyan().bold(), description.dimmed(), note);
    }
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
