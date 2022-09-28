# syntax=docker/dockerfile:1

##
## STEP 1 - BUILD
##
FROM rust:1.62.1 AS base

# specify build working directory
WORKDIR /code

RUN rustup component add rustfmt

# copy packages
RUN cargo init
COPY Cargo.toml .

# vendor third party packages
RUN mkdir -p .cargo \
    && cargo fetch \
    && cargo vendor >> .cargo/config.toml

##
## STEP 2 - BUILD
##
FROM base AS builder

# specify build working directory
WORKDIR /code

COPY templates templates
COPY src src

# compile app
RUN cargo build --release --offline

##
## STEP 3 - DEPLOY
##
FROM debian:bookworm-slim

ARG service_version=unspecified
ARG service_name=unspecified
ARG service_description=unspecified
ARG service_documentation=unspecified
ARG service_homepage=unspecified
ARG service_repository=unspecified
ARG service_license=unspecified
ARG service_build_date=unspecified
ARG service_vcs_ref=unspecified

LABEL maintainer="ffimnsr <ffimnsr@gmail.com>"

LABEL org.opencontainers.image.created="${service_build_date}"
LABEL org.opencontainers.image.url="${service_homepage}"
LABEL org.opencontainers.image.source="${service_repository}"
LABEL org.opencontainers.image.version="${service_version}"
LABEL org.opencontainers.image.revision="${service_vcs_ref}"
LABEL org.opencontainers.image.title="${service_name}"
LABEL org.opencontainers.image.description="${service_description}"
LABEL org.opencontainers.image.documentation="${service_documentation}"
LABEL org.opencontainers.image.licenses="${service_license}"
LABEL org.opencontainers.image.vendor="ffimnsr"
LABEL org.opencontainers.image.authors="ffimnsr <ffimnsr@gmail.com>"

# install ca-certificates
RUN bash -c "apt-get update &> /dev/null \
  && apt-get install -y ca-certificates libssl3 libc6 libgcc1 &> /dev/null \
  && apt-get clean &> /dev/null"

# specify working directory for deployment image
WORKDIR /app

# copy app from build to deployment image
COPY --chown=nobody:nogroup --from=builder /code/target/release/solana-e /app/solana-e

# set user to non-root
USER nobody

# expose server port
EXPOSE 8081

# run the app
ENTRYPOINT [ "/app/solana-e" ]
