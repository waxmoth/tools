FROM alpine:latest

# For more versions, see: https://pkgs.alpinelinux.org/packages?name=jq
ARG version=1.6-r3

RUN apk --update add jq=${version} && \
    rm -rf /var/cache/apk/*

WORKDIR /app
