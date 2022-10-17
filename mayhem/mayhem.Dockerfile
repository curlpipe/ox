# Build Stage
FROM ghcr.io/evanrichter/cargo-fuzz:latest AS builder

## Add source code to the build stage.
ADD . /src
WORKDIR /src

# Compile the shim executable.
RUN cd fuzz && cargo build

# Package Stage
FROM ubuntu:latest

# Create a folder to store the input file.
RUN mkdir /input

# Copy the compiled shim executable from the build stage.
COPY --from=builder /src/fuzz/target/debug/shim /fuzz