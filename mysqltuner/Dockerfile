FROM alpine:latest

ARG version=1.8.3

RUN apk --update add mysql-client perl perl-doc wget && \
    rm -rf /var/cache/apk/*

RUN wget -q https://github.com/major/MySQLTuner-perl/archive/refs/tags/${version}.tar.gz -O /tmp/${version}.tar.gz
RUN tar -xzf /tmp/${version}.tar.gz --directory /tmp
RUN mkdir /app && \
    FILES="mysqltuner.pl basic_passwords.txt vulnerabilities.csv" && \
    for FILE in ${FILES};do cp /tmp/MySQLTuner-perl-${version}/${FILE} /app/; done && \
    rm /tmp/${version}.tar.gz && rm -rf /tmp/MySQLTuner-perl-*

WORKDIR /app

ENTRYPOINT [ "/app/mysqltuner.pl" ]
