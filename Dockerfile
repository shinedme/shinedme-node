# syntax=docker/dockerfile-upstream:experimental
FROM paritytech/ci-linux:production
ENV CARGO_HOME=/var/www/node-template/.cargo
EXPOSE 9944

VOLUME ["/var/www/node-template/"]
WORKDIR /var/www/node-template
COPY . .

RUN --mount=type=cache,target=/var/www/node-template/target \
    --mount=type=cache,target=/var/www/node-template/.cargo/git \
    --mount=type=cache,target=/var/www/node-template/.cargo/registry \
    cargo build --release

ENTRYPOINT [ "./target/release/node-template", "--dev", "--ws-external" ]
