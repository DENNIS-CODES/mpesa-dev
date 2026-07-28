/// Decode an M-Pesa Daraja ResultCode into a human-readable description.
///
/// Returns a `&'static str` with the plain-English meaning, or a generic
/// fallback for unknown codes.
pub fn decode(code: i64) -> &'static str {
    match code {
        0 => "Success",
        1 => "Insufficient funds",
        2 => "Less than minimum transaction value",
        3 => "More than maximum transaction value",
        4 => "Would exceed daily transfer limit",
        5 => "Would exceed minimum balance",
        6 => "Unresolved primary party",
        7 => "Unresolved receiver party",
        8 => "Would exceed maximum balance",
        11 => "Debit account invalid",
        12 => "Credit account invalid",
        13 => "Unresolved debit account",
        14 => "Unresolved credit account",
        15 => "Duplicate detected",
        17 => "Internal failure",
        20 => "Unresolved initiator",
        26 => "Traffic blocking condition in place",
        1001 => "Unable to lock subscriber",
        1002 => "Financial service not allowed",
        1003 => "PIN retries exceeded",
        1004 => "Invalid MSISDN",
        1005 => "PIN length exceeded",
        1006 => "Too early to transact",
        1007 => "Amount too small",
        1008 => "Amount too large",
        1009 => "Failed to debit funds",
        1010 => "Would exceed maximum daily transaction limit",
        1011 => "Incomplete transaction — system error",
        1012 => "Request already exists",
        1013 => "Illegal access: wrong PIN",
        1014 => "Cannot debit a credit account",
        1015 => "Credit to debit not allowed",
        1016 => "M-PESA system under maintenance",
        1017 => "User ineligible for this transaction",
        1019 => "Transaction expired",
        1020 => "Limit breach",
        1021 => "Duplicate for the same account",
        1022 => "Insufficient balance",
        1026 => "Unable to send message to user",
        1032 => "Request cancelled by user",
        1037 => "DS timeout — user cannot be reached",
        2001 => "Wrong credentials provided",
        _ => "Unknown result code",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_codes() {
        assert_eq!(decode(0), "Success");
        assert_eq!(decode(1032), "Request cancelled by user");
        assert_eq!(decode(1037), "DS timeout — user cannot be reached");
        assert_eq!(decode(1), "Insufficient funds");
    }

    #[test]
    fn test_unknown_code() {
        assert_eq!(decode(9999), "Unknown result code");
        assert_eq!(decode(-1), "Unknown result code");
    }
}
