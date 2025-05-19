use anyhow::Result;
use rcp_client_rust::auth::{create_provider, AuthMethod};
use rcp_client_rust::protocol::Client;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure logging
    env_logger::init();

    // Get the current username
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string());

    println!("Testing authentication as user: {}", username);

    // Connect to the server
    println!("Connecting to server...");
    let client = Client::connect("127.0.0.1", 8717).await?;

    // Try all authentication methods
    let methods = vec![AuthMethod::Password, AuthMethod::Psk, AuthMethod::Native];

    for method in methods {
        println!("Testing authentication method: {}", method);

        // Create the authentication provider
        let provider = create_provider(method, &username);

        // Authenticate
        match client.authenticate_with_provider(&*provider).await {
            Ok(true) => println!("  Result: Success"),
            Ok(false) => println!("  Result: Failed"),
            Err(e) => println!("  Result: Error: {}", e),
        }
    }

    println!("Authentication tests complete");

    Ok(())
}
