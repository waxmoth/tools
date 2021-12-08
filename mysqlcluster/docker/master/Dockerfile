FROM mysql:5.7

RUN sed -i '/\[mysqld\]/a server-id=1\nlog-bin' /etc/mysql/mysql.conf.d/mysqld.cnf

COPY entrypoint_extra.sh /docker-entrypoint-initdb.d/
RUN chmod +x /docker-entrypoint-initdb.d/entrypoint_extra.sh
