
import rclpy
from rclpy.node import Node
from sensor_msgs.msg import Image
import numpy as np

def main():
    rclpy.init()
    node = Node('fake_camera_node')
    pub = node.create_publisher(Image, '/camera/color/image_raw', 10)
    
    frame_count = 0
    rate = node.create_rate(30)  # 30 FPS

    print("[OK] Publicando frames sintéticos en /camera/color/image_raw ...")

    while rclpy.ok():
        frame_count += 1

        # Frame RGB sintético: fondo de color que cambia con el tiempo
        img = np.zeros((480, 640, 3), dtype=np.uint8)
        color = frame_count % 255
        img[:, :, 0] = color          # canal R cambia
        img[:, :, 1] = 100            # canal G fijo
        img[:, :, 2] = 200            # canal B fijo

        # Construimos el mensaje a mano, sin cv_bridge
        msg = Image()
        msg.header.stamp = node.get_clock().now().to_msg()
        msg.header.frame_id = "camera_color_optical_frame"
        msg.height = 480
        msg.width = 640
        msg.encoding = "rgb8"
        msg.is_bigendian = False
        msg.step = 640 * 3
        msg.data = img.tobytes()

        pub.publish(msg)

        if frame_count % 30 == 0:
            print(f"[OK] Publicados {frame_count} frames sintéticos")

        rclpy.spin_once(node, timeout_sec=1.0/30.0)

    node.destroy_node()
    rclpy.shutdown()

if __name__ == '__main__':
    main()