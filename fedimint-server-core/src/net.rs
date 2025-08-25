use fedimint_core::module::{ApiEndpointContext, ApiError, ApiResult};

/// A token proving the the API call was authenticated
///
/// Api handlers are encouraged to take it as an argument to avoid sensitive
/// guardian-only logic being accidentally unauthenticated.
pub struct GuardianAuthToken {
    _marker: (), // private field just to make creating it outside impossible
}

impl GuardianAuthToken {
    /// Creates a new auth token for internal use after authentication has been
    /// verified through other means (e.g., dashboard interface).
    ///
    /// WARNING: This should only be called after proper authentication checks
    /// have been performed. Misuse of this constructor bypasses the normal
    /// authentication flow.
    pub fn new_authenticated() -> Self {
        GuardianAuthToken { _marker: () }
    }
}

pub fn check_auth(context: &mut ApiEndpointContext) -> ApiResult<GuardianAuthToken> {
    if context.has_auth() {
        Ok(GuardianAuthToken { _marker: () })
    } else {
        Err(ApiError::unauthorized())
    }
}
