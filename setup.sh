#!/bin/bash
echo "Preparando las dependencias del workspace de ROS 2..."

mkdir -p ros2_ws/src/dependencias
cd ros2_ws/src/dependencias

# Clonar los repositorios base necesarios para rclrs
git clone -b humble https://github.com/ros2/common_interfaces.git
git clone -b humble https://github.com/ros2/example_interfaces.git
git clone -b humble https://github.com/ros2/rcl_interfaces.git
git clone -b humble https://github.com/ros2/rosidl_defaults.git
git clone -b humble https://github.com/ros2/unique_identifier_msgs.git
git clone https://github.com/ros2-rust/rosidl_rust.git

echo "¡Dependencias listas! Ya puedes hacer 'docker compose up -d' y 'colcon build'."