#!/bin/sh
# A bind-mounted ./data is usually created root-owned by Docker. If we start as
# root, make /data writable for the cloudberry user, then drop privileges.
set -eu
if [ "$(id -u)" = 0 ]; then
    # Recursive chown only the first time (existing libraries can be large).
    if [ "$(stat -c %u /data)" != 10001 ]; then
        chown -R cloudberry:cloudberry /data
    fi
    setpriv --reuid=cloudberry --regid=cloudberry --init-groups mkdir -p /data/music
    exec setpriv --reuid=cloudberry --regid=cloudberry --init-groups "$@"
fi
exec "$@"
