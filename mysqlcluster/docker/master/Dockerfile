FROM mysql:5.7

RUN sed -i '/\[mysqld\]/a server-id=1\nlog-bin' /etc/mysql/mysql.conf.d/mysqld.cnf

COPY entrypoint_extra.sh /tmp/
RUN cat /tmp/entrypoint_extra.sh >> /usr/local/bin/docker-entrypoint.sh
