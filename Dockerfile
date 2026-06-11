# ============================================================
# Base: Ubuntu 22.04 + ROS 2 Humble
# ============================================================
FROM ros:humble-ros-base-jammy
ENV DEBIAN_FRONTEND=noninteractive
ENV ROS_DISTRO=humble

# 1. Locales
RUN apt-get update && apt-get install -y locales && \
    locale-gen en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# 2a. Dependencias del sistema base
RUN apt-get update && apt-get install -y \
    build-essential cmake git wget unzip curl pkg-config \
    clang libclang-dev \
    libgtk-3-dev libavcodec-dev libavformat-dev libswscale-dev \
    libboost-all-dev libcurl4-openssl-dev \
    python3-pip python3-vcstool python3-dev python3-numpy \
    && rm -rf /var/lib/apt/lists/*

# 2b. Paquetes ROS
RUN apt-get update && apt-get install -y \
    ros-humble-example-interfaces \
    ros-humble-test-msgs \
    ros-humble-test-interface-files \
    ros-humble-image-view \
    ros-humble-cv-bridge \
    ros-humble-image-transport \
    ros-humble-image-transport-plugins \
    ros-humble-camera-info-manager \
    ros-humble-camera-calibration-parsers \
    ros-humble-image-geometry \
    ros-humble-stereo-msgs \
    ros-humble-diagnostic-updater \
    ros-humble-diagnostic-aggregator \
    ros-humble-diagnostic-msgs \
    ros-humble-self-test \
    ros-humble-realsense2-camera \
    && rm -rf /var/lib/apt/lists/*

# 3. OpenCV 4.11.0 desde fuente
WORKDIR /opt
RUN wget -q -O opencv.zip https://github.com/opencv/opencv/archive/4.11.0.zip \
    && unzip -q opencv.zip \
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

# 4. Variables de entorno OpenCV y Clang
ENV OPENCV_INCLUDE_PATHS="/usr/local/include/opencv4"
ENV LIBCLANG_PATH="/usr/lib/llvm-14/lib/"
ENV PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:/usr/local/lib64/pkgconfig"
ENV LD_LIBRARY_PATH="/usr/local/lib:/usr/local/lib64"
ENV OPENCV_LINK_PATHS="/usr/local/lib:/usr/local/lib64"
ENV OPENCV_LINK_LIBS="opencv_core,opencv_imgproc,opencv_highgui,opencv_videoio,opencv_dnn,opencv_objdetect"

# 5. Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# 6. Plugins colcon para Rust
RUN pip3 install colcon-cargo colcon-ros-cargo

# 7. RealSense SDK
RUN apt-get update && apt-get install -y \
    apt-transport-https software-properties-common gnupg && \
    mkdir -p /etc/apt/keyrings && \
    gpg --keyserver keyserver.ubuntu.com --recv-keys FB0B24895113F120 && \
    gpg --export FB0B24895113F120 > /etc/apt/keyrings/librealsense.pgp && \
    echo "deb [signed-by=/etc/apt/keyrings/librealsense.pgp] https://librealsense.intel.com/Debian/apt-repo jammy main" \
        > /etc/apt/sources.list.d/librealsense.list && \
    apt-get update && \
    apt-get install -y librealsense2-utils librealsense2-dev && \
    rm -rf /var/lib/apt/lists/*

# 8. Shell con ROS sourceado por defecto
RUN echo "source /opt/ros/humble/setup.bash" >> /root/.bashrc && \
    echo "source /opt/ros/humble/setup.bash" >> /root/.profile

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Configurar variables de entorno para el crate opencv de Rust
ENV OPENCV_INCLUDE_PATHS=/usr/local/include/opencv4
ENV OPENCV_LINK_PATHS=/usr/local/lib
ENV OPENCV_LINK_LIBS=opencv_core,opencv_imgproc,opencv_dnn,opencv_videoio,opencv_highgui,opencv_objdetect

# El workspace llega por volumen, no se copia aquí
WORKDIR /app/ros2_ws

ENTRYPOINT ["/entrypoint.sh"]
CMD ["bash"]