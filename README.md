# ROS 2 Humble · Rust · OpenCV 4.11 · YOLOv8 · TrackTrack

Entorno dockerizado para ejecutar un nodo de ROS 2 escrito en Rust que recibe imágenes de una cámara RealSense D435, detecta personas/objetos con YOLOv8 (ONNX) y los trackea en tiempo real con el algoritmo TrackTrack.

Todo corre dentro de Docker para no contaminar el sistema local con las dependencias de ROS 2, OpenCV 4.11 y el toolchain de Rust.

## Arquitectura del pipeline

```text
RealSense D435 (o fake_camera.py)
        │
        │  /camera/camera/color/image_raw  (sensor_msgs/Image)
        ▼
   Nodo Rust (rclrs 0.7)
        │
        ├── OpenCV 4.11 → convierte ROS Image a Mat BGR
        ├── YOLOv8n.onnx → de las carpetas de tracktrack_rust
        └── TrackTrack (mot crate) → tracks con ID persistente
        │
        │  /vision/tracking_result  (sensor_msgs/Image pintada)
        ▼
 Visor Nativo (image_view) con LD_LIBRARY_PATH aislado

```

---

## Estructura del repositorio

```text
ros2_rust_tt/
├── Dockerfile               # Ubuntu 22 + ROS 2 + OpenCV 4.11 + Rust + Boost/Python + Visor
├── docker-compose.yml       # Monta ros2_ws y tracktrack_rust, configura red, USB y X11
├── setup.sh                 # (opcional) clona dependencias extras si las necesitas
├── ros2_ws/
│   ├── fake_camera.py       # Publicador sintético para probar sin cámara real
│   └── src/
│       ├── dependencias/    # common_interfaces, diagnostics, vision_opencv, realsense-ros, etc.
│       │                    # (clonados dentro del Dockerfile, no se suben a Git)
│       └── mis_paquetes/
│           └── mi_nodo/     # Nodo Rust principal (tracktrack)
│               ├── Cargo.toml
│               ├── package.xml
│               └── src/main.rs
└── tracktrack_rust/         # Repo independiente — Ubicación del modelo yolov8n.onnx

```

> `tracktrack_rust` **no es una subcarpeta de este repo**. Es un repositorio independiente que debe clonarse en la raíz para que el contenedor lo monte correctamente.

---

## Puesta en marcha

### Requisitos previos

* Docker y Docker Compose instalados.
* Git.
* **Permisos gráficos en el host:** Ejecutar obligatoriamente `xhost +local:docker` en la terminal local de tu ordenador antes de arrancar los contenedores para permitir que se abran ventanas de vídeo desde Docker.

### 1. Clonar este repositorio

```bash
git clone [https://github.com/alediazcapmany/ros2_rust_tt.git](https://github.com/alediazcapmany/ros2_rust_tt.git)
cd ros2_rust_tt

```

### 2. Clonar la librería de tracking

El archivo `docker-compose.yml` espera la carpeta `tracktrack_rust` compartida en la raíz del proyecto. Descárgala ahí mismo ejecutando:

```bash
git clone [https://github.com/alediazcapmany/tracktrack_rust.git](https://github.com/alediazcapmany/tracktrack_rust.git)

```

### 3. Construir la imagen Docker

La primera vez tarda ~15-20 minutos porque compila todo OpenCV 4.11.0 desde el código fuente original para optimizar el rendimiento de la IA.

```bash
docker compose build dev_env

```

### 4. Compilar el workspace ROS 2

Levanta el servicio con acceso al hardware de la cámara, entra en la terminal del contenedor y realiza la compilación limpia con `colcon`:

```bash
docker compose up -d dev_env_realsense
docker compose exec dev_env_realsense bash

# Dentro del contenedor:
cd /app/ros2_ws
rm -rf build/ install/ log/  # Limpieza de cachés previas si existen
colcon build
source install/setup.bash

```

---

## Ejecución con Hardware Real (RealSense D435)

### Paso Previo — Permisos Gráficos

Antes de levantar nada, dale permisos a Docker para que pueda abrir ventanas en tu escritorio. Ejecuta esto en una terminal de **tu ordenador (fuera de Docker)**:

```bash
xhost +local:root

```

### Terminal 1 — Driver de la Cámara Intel

Entra al contenedor y arranca el wrapper de ROS 2. Como estamos usando un puerto **USB 3.1**, tenemos ancho de banda suficiente para resoluciones altas y fluidez, lo cual es crítico para no perder la trayectoria.

```bash
docker compose exec dev_env_realsense bash
source /opt/ros/humble/setup.bash
cd /app/ros2_ws && source install/setup.bash

# Opción A: Parámetros por defecto con IMU activa (negocia automáticamente resolución y FPS)
ros2 launch realsense2_camera rs_launch.py enable_gyro:=true enable_accel:=true

# Opción B: Forzar 60 FPS (Recomendado para seguimiento rápido)
ros2 launch realsense2_camera rs_launch.py depth_module.depth_profile:=848x480x60 rgb_camera.color_profile:=848x480x60 enable_gyro:=true enable_accel:=true

```

### Terminal 2 — Nodo de Tracking (Rust)

Arranca el nodo principal encargado de procesar las imágenes, pasar el modelo de inferencia y enlazar los IDs de seguimiento:

```bash
docker compose exec dev_env_realsense bash
source /opt/ros/humble/setup.bash
cd /app/ros2_ws && source install/setup.bash

ros2 run mi_nodo mi_nodo

```

### Terminal 3 — Visualización Gráfica en Directo

El nodo publica el vídeo procesado en el tópico `/vision/tracking_result`. Para abrir el visor nativo sin provocar fallos de segmentación (*Segmentation fault*) por conflictos entre la versión de OpenCV de Ubuntu y la nuestra compilada a medida, aislamos el path de las librerías:

```bash
docker compose exec dev_env_realsense bash
source /opt/ros/humble/setup.bash

LD_LIBRARY_PATH=/opt/ros/humble/lib ros2 run image_view image_view --ros-args -r image:=/vision/tracking_result

```

---

## Configuración del Modelo YOLOv8

El nodo de Rust busca el modelo de red neuronal en la ruta global `/app/yolov8n.onnx`. Al clonar el repositorio secundario, dicho archivo ya se encuentra descargado dentro de la ruta `/app/tracktrack_rust/yolov8n.onnx`.

Para que el nodo lo localice al arrancar sin necesidad de modificar rutas internas de código, basta con generar un enlace simbólico rápido dentro del contenedor antes de ejecutarlo:

```bash
ln -s /app/tracktrack_rust/yolov8n.onnx /app/yolov8n.onnx

```

---

## Tabla de Versiones Clave

| Componente | Versión Utilizada |
| --- | --- |
| **ROS 2** | Humble Hawksbill (Ubuntu 22.04 LTS) |
| **Toolchain de Rust** | Estable (`stable`) |
| **OpenCV (C++)** | 4.11.0 (Nativa, optimizada con GTK) |
| **OpenCV (Rust Crate)** | v0.94.4 |
| **RealSense SDK** | v2.58.1 |
| **RealSense ROS Wrapper** | v4.57.0 |

---

## Solución de Problemas Frecuentes

* **`Can't read ONNX file` / Fallo `StsBadArg`:** El código de Rust no encuentra el archivo del modelo en la raíz de `/app/`. Asegúrate de haber ejecutado correctamente el paso del enlace simbólico (`ln -s`).
* **`Authorization required` / Fallo de GTK al visualizar:** El contenedor no tiene permiso para renderizar ventanas en tu escritorio local de Ubuntu. Recuerda abrir una terminal en tu sistema host físico y ejecutar `xhost +local:docker` o `xhost +local:root`.
* **`Could NOT find Boost (missing: python3)`:** Resuelto de forma permanente en esta imagen de Docker utilizando el paquete completo `libboost-all-dev` junto a las cabeceras nativas de desarrollo `python3-dev` y las librerías matriciales de `python3-numpy` en el proceso de construcción.

```