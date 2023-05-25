FROM rust:1.69 as build
ARG binary
WORKDIR /usr/src/aws_lambda
COPY aws_lambda .
COPY src ./surrealdb/src
COPY lib ./surrealdb/lib
COPY Cargo.toml ./surrealdb/
COPY build.rs ./surrealdb/
RUN apt-get -y update && \
    apt-get -y install curl llvm cmake binutils clang-11 qemu-user musl-tools libssl-dev pkg-config build-essential protobuf-compiler
RUN cargo install --bin ${binary} --path .

FROM gcr.io/distroless/cc-debian11
ARG binary
ARG log_level
ENV RUST_LOG=${log_level}
COPY --from=build /usr/local/cargo/bin/${binary} /asset-output/bootstrap
ENTRYPOINT [ "/asset-output/bootstrap"  ]
