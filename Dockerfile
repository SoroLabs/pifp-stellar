FROM rust:1.81

# Install required system tools
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Add WebAssembly target
RUN rustup target add wasm32-unknown-unknown

# Install Soroban CLI
# Using the recommended installation method for stellar-cli (formerly soroban-cli)
RUN cargo install --locked stellar-cli@22.0.1 

# Set working directory
WORKDIR /workspace

# Default command to keep container running in the background for dev
CMD ["tail", "-f", "/dev/null"]
