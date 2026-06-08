# Default recipe: list all available commands
default:
    @just --list

# Build the server binary (defaults to release mode)
build mode="release":
    cargo build -p server {{ if mode == "release" { "--release" } else { "" } }}

# Deploy the server binary as a symlink to ~/.local/bin/chat-server
deploy dest_name="chat-server" mode="release": (build mode)
    mkdir -p "$HOME/.local/bin"
    ln -sf "{{justfile_directory()}}/target/{{mode}}/server" "$HOME/.local/bin/{{dest_name}}"
    @echo "Server successfully deployed to ~/.local/bin/{{dest_name}}!"

# Build the client-tui binary (defaults to release mode)
build-client mode="release":
    cargo build -p client-tui {{ if mode == "release" { "--release" } else { "" } }}

# Deploy the client-tui binary as a symlink to ~/.local/bin/chat-client
deploy-client dest_name="chat-client" mode="release": (build-client mode)
    mkdir -p "$HOME/.local/bin"
    ln -sf "{{justfile_directory()}}/target/{{mode}}/client-tui" "$HOME/.local/bin/{{dest_name}}"
    @echo "Client successfully deployed to ~/.local/bin/{{dest_name}}!"

# Run the server in development mode (debug)
run-server *args="":
    cargo run -p server -- {{args}}

# Run the client in development mode (debug)
run-client *args="":
    cargo run -p client-tui -- {{args}}

# Clean cargo build artifacts
clean:
    cargo clean
