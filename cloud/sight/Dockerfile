# The locator as a Cloudflare Container: `sight serve` with the elevation grid and horizon
# fields baked in. Built by `wrangler deploy` (Docker must be running) from the repository root,
# which is the build context (see wrangler.jsonc and /.dockerignore).
#
# Containers run linux/amd64. The build stage runs on the builder's own architecture and
# cross-compiles to x86_64, so an Apple-silicon Mac compiles natively instead of under emulation.

FROM --platform=$BUILDPLATFORM rust:1-bookworm AS build
RUN if [ "$(uname -m)" != "x86_64" ]; then \
      apt-get update && apt-get install -y --no-install-recommends gcc-x86-64-linux-gnu libc6-dev-amd64-cross \
      && rm -rf /var/lib/apt/lists/*; \
    fi \
    && rustup target add x86_64-unknown-linux-gnu
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
# x86-64-v3 (AVX2, FMA): the screen's inner loops are fused multiply-adds, and on baseline x86-64
# each `mul_add` becomes a call into a software fma, which made a search ~36 s instead of seconds.
# Every current server x86 CPU (Cloudflare's included) supports v3.
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C target-cpu=x86-64-v3"
WORKDIR /src
COPY sight/ ./
RUN cargo build --release --locked --target x86_64-unknown-linux-gnu --bin sight

FROM --platform=linux/amd64 debian:bookworm-slim
# heif-convert decodes iPhone HEIC uploads.
RUN apt-get update && apt-get install -y --no-install-recommends libheif-examples \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/x86_64-unknown-linux-gnu/release/sight /usr/local/bin/sight
COPY data/elevation/envi/n41w112 data/elevation/envi/n41w112.hdr /data/
COPY data/elevation/field/n41w112.hzn data/elevation/field/n41w112-rough500.hzn /data/
EXPOSE 4373
CMD ["sight", "serve", "/data/n41w112", "/data/n41w112.hzn,/data/n41w112-rough500.hzn", "--host", "0.0.0.0", "--port", "4373"]
