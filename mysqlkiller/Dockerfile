FROM alpine:latest

RUN apk --update add mysql-client bash && \
    rm -rf /var/cache/apk/*

ADD bin /app/bin

RUN chmod a+x /app/bin/*.sh

WORKDIR /app
