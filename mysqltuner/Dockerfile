FROM alpine:latest

RUN apk --update add mysql-client perl perl-doc && \
    rm -rf /var/cache/apk/*

RUN wget http://mysqltuner.pl/ -O mysqltuner.pl
RUN wget https://raw.githubusercontent.com/major/MySQLTuner-perl/master/basic_passwords.txt -O basic_passwords.txt
RUN wget https://raw.githubusercontent.com/major/MySQLTuner-perl/master/vulnerabilities.csv -O vulnerabilities.csv

ENTRYPOINT [ "/mysqltuner.pl" ]
