use serde::{Deserialize, Serialize};

/// Response body from `GET /oauth/v1/generate?grant_type=client_credentials`.
///
/// Daraja returns `expires_in` as a numeric string (e.g. `"3599"`), not a
/// number, so it is deserialized as `String` and parsed by the caller.
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub expires_in: String,
}

/// Error body Daraja returns on most non-2xx responses.
#[derive(Debug, Clone, Deserialize)]
pub struct DarajaErrorResponse {
    /// Not surfaced anywhere yet, but useful when filing a support ticket
    /// with Safaricom, so kept on the struct even though nothing reads it.
    #[allow(dead_code)]
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

impl DarajaErrorResponse {
    pub fn description(&self) -> String {
        match (&self.error_code, &self.error_message) {
            (Some(code), Some(msg)) => format!("{code}: {msg}"),
            (None, Some(msg)) => msg.clone(),
            _ => "unknown Daraja error".to_string(),
        }
    }
}

/// Body for `POST /mpesa/stkpush/v1/processrequest` (Lipa na M-Pesa Online).
#[derive(Debug, Clone, Serialize)]
pub struct StkPushRequest {
    #[serde(rename = "BusinessShortCode")]
    pub business_short_code: String,
    #[serde(rename = "Password")]
    pub password: String,
    #[serde(rename = "Timestamp")]
    pub timestamp: String,
    #[serde(rename = "TransactionType")]
    pub transaction_type: String,
    #[serde(rename = "Amount")]
    pub amount: u32,
    #[serde(rename = "PartyA")]
    pub party_a: String,
    #[serde(rename = "PartyB")]
    pub party_b: String,
    #[serde(rename = "PhoneNumber")]
    pub phone_number: String,
    #[serde(rename = "CallBackURL")]
    pub callback_url: String,
    #[serde(rename = "AccountReference")]
    pub account_reference: String,
    #[serde(rename = "TransactionDesc")]
    pub transaction_desc: String,
}

/// Response body from a successful STK push request submission.
///
/// Note this only confirms the *request* was accepted; the actual payment
/// result arrives later as an asynchronous callback (see `inspect`).
#[derive(Debug, Clone, Deserialize)]
pub struct StkPushResponse {
    /// Kept on the struct to match the real API shape; `replay` (Milestone 4)
    /// will need it to correlate a stored callback back to its push request.
    #[allow(dead_code)]
    #[serde(rename = "MerchantRequestID")]
    pub merchant_request_id: String,
    #[serde(rename = "CheckoutRequestID")]
    pub checkout_request_id: String,
    #[serde(rename = "ResponseCode")]
    pub response_code: String,
    #[serde(rename = "ResponseDescription")]
    pub response_description: String,
    #[allow(dead_code)]
    #[serde(rename = "CustomerMessage")]
    pub customer_message: String,
}
