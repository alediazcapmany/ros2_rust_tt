use opencv::core;
use rclrs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializamos el contexto de ROS 2
    let context = Context::default_from_env()?;

    // 2. Creamos el executor y el nodo desde él
    let executor = context.create_basic_executor();
    let _node = executor.create_node("nodo_vision")?;

    println!("✅ Nodo de ROS 2 inicializado correctamente en Rust!");

    // 3. Comprobamos OpenCV
    println!("✅ Versión de OpenCV detectada: {}", core::CV_VERSION);

    Ok(())
}