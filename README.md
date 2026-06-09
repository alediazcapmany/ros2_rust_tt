# 🦀 ROS 2 Humble + Rust + OpenCV 4.11 en Docker

He preparado este entorno dockerizado para poder programar nodos de ROS 2 en Rust usando visión artificial con OpenCV, sin tener que ensuciar mi sistema operativo local ni volverme loco con las dependencias de C++.

El entorno está totalmente blindado. Si quieres probarlo o usarlo de base, solo tienes que seguir estos pasos.

## 1. Descargar el proyecto

Primero clona este repositorio en tu ordenador y entra en la carpeta:

```bash
git clone https://github.com/TU_USUARIO/ros2_rust_tt.git
cd ros2_rust_tt

```

## 2. Descargar las dependencias base

Para no sobrecargar el control de versiones, las dependencias y mensajes de ROS 2 necesarios para `rclrs` están ignorados en Git. He dejado un script preparado que se encarga de clonar todo lo necesario en su sitio correcto:

```bash
chmod +x setup.sh
./setup.sh

```

## 3. Añadir la librería de visión (TrackTrack)

El proyecto está pensado para integrarse con mi librería de visión. El `docker-compose` espera encontrarla en una carpeta llamada `tracktrack_rust`. Descárgala ahí mismo ejecutando:

```bash
git clone https://github.com/alediazcapmany/tracktrack_rust.git

```

## 4. Levantar el contenedor

Ahora le toca currar a Docker. El siguiente comando construirá la imagen base instalando Ubuntu, Rust, ROS 2 y compilando OpenCV 4.11 desde el código fuente. La primera vez tardará un rato, así que ten paciencia.

```bash
docker compose up -d --build

```

## 5. Compilar el Workspace

En cuanto el contenedor termine de construirse y esté corriendo en segundo plano, nos metemos dentro de su terminal interactiva:

```bash
docker compose exec dev_env bash

```

Una vez dentro (verás que estás en la ruta `/app`), compilamos el espacio de trabajo de ROS 2 de forma normal:

```bash
cd ros2_ws
colcon build

```

## 6. Probar el nodo

Si el proceso de `colcon build` ha terminado en verde, ya solo queda cargar las variables de entorno y lanzar el binario:

```bash
source install/setup.bash
ros2 run mi_nodo mi_nodo

```

Si todo ha ido bien, verás un mensaje confirmando que el nodo de ROS 2 se ha levantado en Rust y te mostrará por pantalla la versión de OpenCV detectada. ¡A programar!
