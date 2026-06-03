# Usamos Ubuntu 22.04 con ROS 2 Humble ya instalado
FROM ros:humble-ros-base-jammy

ENV DEBIAN_FRONTEND=noninteractive
ENV ROS_DISTRO=humble

# 1. Configurar los locales (sacado de tu repo)
RUN apt-get update && apt-get install -y locales && \
    locale-gen en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# 2. Instalar dependencias del sistema, herramientas de compilación para OpenCV y workarounds de ROS
RUN apt-get update && apt-get install -y \
    build-essential cmake git wget unzip curl pkg-config \
    clang libclang-dev \
    libgtk-3-dev libavcodec-dev libavformat-dev libswscale-dev \
    python3-pip python3-vcstool \
    ros-humble-example-interfaces ros-humble-test-msgs ros-humble-test-interface-files \
    && rm -rf /var/lib/apt/lists/*

# 3. Descargar y compilar OpenCV 4.11.0 (Solo tus módulos necesarios)
WORKDIR /opt
RUN wget -O opencv.zip https://github.com/opencv/opencv/archive/4.11.0.zip \
    && unzip opencv.zip \
    && mkdir -p build && cd build \
    && cmake ../opencv-4.11.0 \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DBUILD_LIST=core,dnn,highgui,imgproc,videoio,objdetect \
        -DWITH_GTK=ON \
        -DBUILD_TESTS=OFF \
        -DBUILD_PERF_TESTS=OFF \
    && make -j$(nproc) \
    && make install \
    && ldconfig \
    && rm -rf /opt/opencv* /opt/build

# 4. Variables de entorno apuntando a OpenCV (local) y Clang (llvm-14 para Ubuntu 22)
ENV OPENCV_INCLUDE_PATHS="/usr/local/include/opencv4"
ENV LIBCLANG_PATH="/usr/lib/llvm-14/lib/"
ENV PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/lib64/pkgconfig"

# 5. Instalar Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# 6. Instalar los plugins de colcon para Rust
RUN pip3 install colcon-cargo colcon-ros-cargo

# 7. Preparar el workspace de ROS y clonar dependencias base de Humble
WORKDIR /app/ros2_ws
RUN mkdir -p src && cd src && \
    git clone -b humble https://github.com/ros2/common_interfaces.git && \
    git clone -b humble https://github.com/ros2/example_interfaces.git && \
    git clone -b humble https://github.com/ros2/rcl_interfaces.git && \
    git clone -b humble https://github.com/ros2/rosidl_defaults.git && \
    git clone -b humble https://github.com/ros2/unique_identifier_msgs.git && \
    git clone https://github.com/ros2-rust/rosidl_rust.git

# Añadimos el source de ROS al bashrc
RUN echo "source /opt/ros/humble/setup.bash" >> /root/.bashrc

# Dejamos el WORKDIR donde tenías tu repo
WORKDIR /app
CMD ["/bin/bash"]