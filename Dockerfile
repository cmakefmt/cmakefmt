# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --locked && strip target/release/cmakefmt

FROM alpine:3

COPY --from=builder /build/target/release/cmakefmt /usr/local/bin/cmakefmt

ENTRYPOINT ["cmakefmt"]
CMD ["--help"]
