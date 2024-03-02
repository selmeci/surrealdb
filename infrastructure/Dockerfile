FROM rust:1.76 as base
# Install required dependencies
RUN apt-get -y update && \
    apt-get -y upgrade && \
    apt-get -y install curl llvm cmake binutils clang qemu-user musl-tools libssl-dev pkg-config build-essential protobuf-compiler

FROM base as build

WORKDIR /usr/src/surreal

# Copy the source code
COPY . .

# Build and install SurrealDB
RUN cargo install --bin surreal --path /usr/src/surreal

FROM rust:1.76 as surrealdb

COPY --from=build /usr/local/cargo/bin/surreal /usr/bin

EXPOSE 8000

ENTRYPOINT ["/usr/bin/surreal"]
