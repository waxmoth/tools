#!/bin/bash

export MYSQL_PWD=${MYSQL_ROOT_PASSWORD}
mysql -uroot -e \
      "
      STOP SLAVE;
      CHANGE MASTER TO master_host='${MYSQL_MASTER_SERVICE_HOST}', master_user='${MYSQL_REPLICATION_USER}', master_password='${MYSQL_REPLICATION_PASSWORD}';
      RESET SLAVE;
      START SLAVE;
      "
