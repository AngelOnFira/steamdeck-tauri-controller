#!/bin/bash

echo "Generating Tauri updater signing keys..."
echo "You will be prompted to enter a password for the private key."
echo ""

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo is not installed. Please install Rust first."
    exit 1
fi

# Check if tauri-cli is installed
if ! cargo tauri --version &> /dev/null 2>&1; then
    echo "Installing Tauri CLI..."
    cargo install tauri-cli
fi

# Generate the keys
cargo tauri signer generate -w .tauri/myapp.key

echo ""
echo "Keys generated successfully!"
echo ""
echo "The private key has been saved to: .tauri/myapp.key"
echo "The public key has been saved to: .tauri/myapp.key.pub"
echo ""
echo "IMPORTANT: Add the following to your GitHub repository secrets:"
echo "1. TAURI_SIGNING_PRIVATE_KEY - Contents of .tauri/myapp.key"
echo "2. TAURI_SIGNING_PRIVATE_KEY_PASSWORD - The password you just entered"
echo ""
echo "Then add the public key to your tauri.conf.json in the updater plugin config."
echo ""
echo "Your public key is:"
cat .tauri/myapp.key.pub