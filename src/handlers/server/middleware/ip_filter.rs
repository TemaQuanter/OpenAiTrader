use std::net::SocketAddr;

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

/// This is a middleware that restricts the access to the server, so that only
/// whitelisted addresses can interact with it.
pub async fn _ip_filter_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    // Define the allowed IP address
    let allowed_ip = "256.256.256.256";

    // Check if the source IP address matches the allowed IP
    if let Some(peer_addr) = req.extensions().get::<SocketAddr>() {
        if peer_addr.ip().to_string() == allowed_ip {
            // Allow the request to proceed
            Ok(next.run(req).await)
        } else {
            // Reject the request
            Ok(Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body("Access denied".into())
                .unwrap())
        }
    } else {
        // If IP couldn't be determined, reject the request
        Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body("Access denied".into())
            .unwrap())
    } // end if
} // end fn ip_filter_middleware
