#!/usr/bin/env bash
# Monitor the Mysql replication delay and send email

MYSQL_USER=${MYSQL_USER:-'root'}
MYSQL_HOST=${MYSQL_HOST:-'127.0.0.1'}
MAX_REPLICATION_DELAY=${MAX_REPLICATION_DELAY:-300}
EMAIL_TO=${EMAIL_TO:-''}
EMAIL_FROM=${EMAIL_FROM:-'noreply@example.com'}
SMTP_HOST=${SMTP_HOST:-'127.0.0.1'}

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
    sendmail -i -t "${EMAIL_TO}" -f "${EMAIL_FROM}" -S "${SMTP_HOST}" << EOL
Subject: [ALERT] DB replication!

MySQL replication delay, please check and fix it!

MySQL Monitor
EOL
    exit 1
fi

log 'Done!'
