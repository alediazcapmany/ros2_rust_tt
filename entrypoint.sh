#!/bin/bash
set -e

source /opt/ros/humble/setup.bash

# Si el workspace ya está compilado, sourcearlo automáticamente
if [ -f /app/ros2_ws/install/setup.bash ]; then
    source /app/ros2_ws/install/setup.bash
fi

exec "$@"