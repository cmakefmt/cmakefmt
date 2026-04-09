# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

FROM rust:1-slim AS builder

WORKDIR /build
COPY . .
RUN cargo build --release --locked && strip target/release/cmakefmt

FROM debian:bookworm-slim

COPY --from=builder /build/target/release/cmakefmt /usr/local/bin/cmakefmt

ENTRYPOINT ["cmakefmt"]
CMD ["--help"]
