/// Whether a ResultCode represents a completed payment or some kind of
/// failure. Used purely to pick a color when printing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure,
}

/// Translates a Daraja `ResultCode` into a plain-English description and
/// whether it represents success or failure.
///
/// Compiled from the Daraja API docs and real sandbox callback samples.
/// Codes not in this table still get a readable fallback rather than a
/// bare number.
pub fn describe(code: i64) -> (Outcome, &'static str) {
    match code {
        0 => (Outcome::Success, "Success — the transaction completed"),
        1 => (
            Outcome::Failure,
            "Insufficient funds — the customer doesn't have enough M-Pesa balance",
        ),
        17 => (
            Outcome::Failure,
            "System internal error at Safaricom — safe to retry",
        ),
        26 => (
            Outcome::Failure,
            "System busy — safe to retry after a short wait",
        ),
        1001 => (
            Outcome::Failure,
            "Unable to lock subscriber — another transaction is already in progress on this number",
        ),
        1019 => (
            Outcome::Failure,
            "Transaction expired — the customer took too long to respond",
        ),
        1025 => (
            Outcome::Failure,
            "Unable to initiate the STK push — check TransactionDesc length and that the number is a valid Safaricom line",
        ),
        1032 => (
            Outcome::Failure,
            "Cancelled — the customer pressed cancel on the STK prompt",
        ),
        1037 => (
            Outcome::Failure,
            "Timeout — the customer's phone was unreachable or didn't respond in time",
        ),
        2001 => (
            Outcome::Failure,
            "Wrong PIN — the customer entered an incorrect M-Pesa PIN",
        ),
        9999 => (
            Outcome::Failure,
            "Unknown error from Safaricom — check ResultDesc for details",
        ),
        _ => (
            Outcome::Failure,
            "Unrecognized ResultCode — check ResultDesc for details",
        ),
    }
}
