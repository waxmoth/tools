FROM mysql:8.0

COPY conf/my.cnf /etc/mysql/my.cnf

COPY entrypoint_extra.sh /docker-entrypoint-initdb.d/
RUN chmod +x /docker-entrypoint-initdb.d/entrypoint_extra.sh
