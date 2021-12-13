#!/usr/bin/env bash
# Kill sleeping and long run threads

MYSQL_USER=${MYSQL_USER:-'root'}
MYSQL_HOST=${MYSQL_HOST:-'127.0.0.1'}
MAX_RUN_TIME=${MAX_RUN_TIME:-3000}
CHECK_DB_USERS=${MYSQL_USERS:-"'root'"}

export MYSQL_PWD=${MYSQL_PWD:-root}

log() {
    echo $(date +"%Y-%m-%d %H:%M:%S|")"$1";
}

get_thread_sql() {
	local threadId=$1;
	echo $(
        mysql \
            -u "${MYSQL_USER}" \
            -h "${MYSQL_HOST}" \
            -N \
            -e \
            "
            SELECT SQL_TEXT FROM performance_schema.events_statements_current WHERE THREAD_ID = ${threadId};
            "
    )
    return;
}

log 'Kill sleeping and long run DB connections';

threadIds=$( \
    mysql \
        -u "${MYSQL_USER}" \
        -h "${MYSQL_HOST}" \
        -N \
        -e \
        "
        SELECT ID FROM INFORMATION_SCHEMA.PROCESSLIST WHERE USER IN (${CHECK_DB_USERS}) AND COMMAND IN ('Sleep', 'Query') AND Time > ${MAX_RUN_TIME} ORDER BY Time DESC;
        "
)

log 'Get thread ids: '"${threadIds}";

for threadId in ${threadIds}; do
    log 'Kill thread: '${threadId}'|Query: '$(get_thread_sql ${threadId})
    # Note: You can run 'CALL mysql.rds_kill(${threadId});' for AWS RDS
    mysql \
        -u "${MYSQL_USER}" \
        -h "${MYSQL_HOST}" \
        -N \
        -e \
        "
        KILL ${threadId};
        "
done

log 'Done!'
