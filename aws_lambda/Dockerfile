FROM rust:1.70 as build
RUN apt-get -y update && \
    apt-get -y install curl llvm cmake binutils clang-11 qemu-user musl-tools libssl-dev pkg-config build-essential protobuf-compiler && \
    rustup target add aarch64-unknown-linux-gnu

WORKDIR /usr/src/aws_lambda
COPY aws_lambda .
COPY src ./surrealdb/src
COPY lib ./surrealdb/lib
COPY Cargo.toml ./surrealdb/
COPY build.rs ./surrealdb/

RUN cargo install --target aarch64-unknown-linux-gnu --bin surrealdb --path .

FROM gcr.io/distroless/cc-debian11
COPY --from=build /usr/local/cargo/bin/surrealdb /asset-output/bootstrap
ENTRYPOINT [ "/asset-output/bootstrap" ]
