FROM mysql:5.7

RUN RAND="$(date +%s | rev | cut -c 1-2)$(echo ${RANDOM})" && \
    sed -i '/\[mysqld\]/a server-id='$RAND'\nread_only=1\nlog-bin' /etc/mysql/mysql.conf.d/mysqld.cnf

COPY entrypoint_extra.sh /docker-entrypoint-initdb.d/
RUN chmod +x /docker-entrypoint-initdb.d/entrypoint_extra.sh
