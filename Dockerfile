FROM rust

COPY . /var/canna-scrape-web/
WORKDIR /var/canna-scrape-web/
RUN rustup override set nightly-2018-02-09
RUN cargo update
RUN cargo build --release
EXPOSE 8000
ENV ROCKET_ENV=production

ENTRYPOINT ["cargo", "run", "--release"]
