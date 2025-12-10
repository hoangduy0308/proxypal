# Stage 1: Build frontend
FROM node:20-alpine AS frontend

WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy package files
COPY package.json pnpm-lock.yaml ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy frontend source
COPY index.html ./
COPY vite.config.ts tsconfig.json tsconfig.node.json ./
COPY tailwind.config.js postcss.config.js ./
COPY public ./public
COPY src ./src

# Build frontend
RUN pnpm build

# Stage 2: Build backend
FROM rust:1.87 AS backend

WORKDIR /app

# Copy Cargo workspace files
COPY Cargo.toml Cargo.lock ./
COPY src-tauri ./src-tauri
COPY proxypal-server ./proxypal-server

# Copy frontend dist from previous stage
COPY --from=frontend /app/dist ./dist

# Build release binary
RUN cargo build --release -p proxypal-server

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy proxypal-server binary
COPY --from=backend /app/target/release/proxypal-server ./proxypal-server

# Copy CLIProxyAPI binary
COPY src-tauri/binaries/cliproxyapi-x86_64-unknown-linux-gnu ./cliproxyapi
RUN chmod +x ./cliproxyapi

# Copy frontend dist
COPY --from=frontend /app/dist ./dist

# Create data directory
RUN mkdir -p /data

# Set environment defaults
ENV PORT=3000
ENV DATABASE_PATH=/data/proxypal.db
ENV DATA_DIR=/data
ENV CLIPROXY_BINARY_PATH=/app/cliproxyapi

EXPOSE 3000

CMD ["./proxypal-server"]
