# syntax=docker/dockerfile-upstream:experimental
FROM paritytech/ci-linux:production
# ENV RUSTUP_HOME=/var/www/node-template/.rustup
ENV CARGO_HOME=/var/www/node-template/.cargo
EXPOSE 9944

VOLUME ["/var/www/node-template/"]
WORKDIR /var/www/node-template
COPY . .

RUN cargo build --release

CMD [ "./target/release/node-template", "--dev", "--ws-external" ]
