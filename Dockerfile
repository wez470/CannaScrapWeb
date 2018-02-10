FROM rust

COPY . /var/canna-scrape-web/
WORKDIR /var/canna-scrape-web/
RUN rustup override set nightly
RUN cargo update
RUN cargo build --release
EXPOSE 8000
ENV ROCKET_ENV=production
RUN rm -rf src Cargo.lock Cargo.toml Dockerfile Rocket.toml 

ENTRYPOINT ["cargo", "run", "--release"]
