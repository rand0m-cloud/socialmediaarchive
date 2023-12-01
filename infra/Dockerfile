FROM rust:alpine AS builder
ADD backend /code
RUN apk add --no-cache musl-dev openssl-dev
RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/code/target \
  RUSTFLAGS="-C target-feature=-crt-static" cargo install --path /code

# runtime container
FROM alpine

# add dependencies
RUN apk add --no-cache libgcc python3 py3-pip ffmpeg
RUN python3 -m pip install yt-dlp

COPY --from=builder /usr/local/cargo/bin/backend /usr/local/bin/backend

RUN mkdir /app
WORKDIR /app

CMD ["/usr/local/bin/backend", "daemon"]
