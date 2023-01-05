#!/usr/bin/env bash
# Monitor the Mysql replication delay and send email

MYSQL_USER=${MYSQL_USER:-'root'}
MYSQL_HOST=${MYSQL_HOST:-'127.0.0.1'}
MAX_REPLICATION_DELAY=${MAX_REPLICATION_DELAY:-300}
EMAIL_TO=${EMAIL_TO:-''}
EMAIL_FROM=${EMAIL_FROM:-'noreply@example.com'}

export MYSQL_PWD=${MYSQL_PWD:-root}

log() {
    date +"%Y-%m-%d %H:%M:%S|$1";
}

secondsDelay=$(
    mysql \
        -u "${MYSQL_USER}" \
        -h "${MYSQL_HOST}" \
        -e \
        "
        SHOW REPLICA STATUS\G;
        " \
    | grep Seconds_Behind_Source | awk '{print $2}'
)

log "Checking the DB replication delay: ${secondsDelay}"

if [[ ${secondsDelay} -gt ${MAX_REPLICATION_DELAY} ]]; then
    /usr/sbin/sendmail -i -t -v "${EMAIL_TO}" -f "${EMAIL_FROM}" << EOL
Subject: [ALERT] DB replication!

DB replication delay, please check and fix it!
EOL
fi

log 'Done!'
