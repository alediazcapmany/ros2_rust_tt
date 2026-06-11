use mot::tracktrack::track::{Detection, TrackState};
use mot::tracktrack::tracker::{Args, Tracker};
use opencv::{
    core::{self, Mat, Rect, Scalar, Size, Vector},
    dnn, imgproc,
    prelude::*,
};
use rclrs::*;
use sensor_msgs::msg::Image as RosImage;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;

// ============================================================
// Hiperparámetros YOLO
// ============================================================
const YOLO_MODEL_PATH: &str = "/app/tracktrack_rust/yolov8n.onnx";
const INPUT_SIZE: i32 = 640;
const CONF_THRESHOLD: f32 = 0.3;
const NMS_THRESHOLD: f32 = 0.4;
const SCALE: f64 = 0.75; // Reducimos el frame a la mitad antes de YOLO

// ============================================================
// Clases COCO a detectar
// 0=person  41=cup (taza/copa)
// ============================================================
const CLASES_OBJETIVO: &[usize] = &[0, 41];

// Pasar imagen de ROS a matriz para OpenCV
fn image_to_mat(msg: &RosImage) -> Result<Mat, opencv::Error> {
    let n_pixels = (msg.height * msg.width) as usize;
    let pixels: &[core::Vec3b] =
        unsafe { std::slice::from_raw_parts(msg.data.as_ptr() as *const core::Vec3b, n_pixels) };
    let raw = core::Mat::new_rows_cols_with_data(msg.height as i32, msg.width as i32, pixels)?
        .try_clone()?;
    let mut bgr = Mat::default();
    imgproc::cvt_color(
        &raw,
        &mut bgr,
        imgproc::COLOR_RGB2BGR,
        0,
        core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;
    Ok(bgr)
}

// Pasar Matriz de OpenCV a imagen de ROS
fn mat_to_image(mat: &Mat, frame_id: &str) -> Result<RosImage, opencv::Error> {
    let mut rgb = Mat::default();
    imgproc::cvt_color(
        mat,
        &mut rgb,
        imgproc::COLOR_BGR2RGB,
        0,
        core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    let size = rgb.size()?;
    let step = rgb.step1(0)?;
    let data_len = (size.height as usize) * step;

    let mut data = vec![0u8; data_len];
    let src_data = rgb.data_bytes()?;
    data.copy_from_slice(src_data);

    let mut msg = RosImage::default();
    msg.header.frame_id = frame_id.to_string();
    msg.height = size.height as u32;
    msg.width = size.width as u32;
    msg.encoding = "rgb8".to_string();
    msg.is_bigendian = 0;
    msg.step = step as u32;
    msg.data = data;

    Ok(msg)
}

// Función de inferencia con YOLO para detectar imágenes
// Modificamos la firma para que devuelva un f64 extra con los milisegundos de la inferencia pura
fn detect_yolo(
    net: &mut dnn::Net,
    frame: &Mat,
) -> Result<(Vec<Detection>, Vec<usize>, f64), opencv::Error> {
    let inv_scale = (1.0 / SCALE) as f32;

    // --- 1. PREPROCESAMIENTO ---
    let mut frame_small = Mat::default();
    imgproc::resize(
        frame,
        &mut frame_small,
        Size::new(0, 0),
        SCALE,
        SCALE,
        imgproc::INTER_AREA,
    )?;

    let blob = dnn::blob_from_image(
        &frame_small,
        1.0 / 255.0,
        Size::new(INPUT_SIZE, INPUT_SIZE),
        Scalar::default(),
        true,
        false,
        core::CV_32F,
    )?;
    net.set_input(&blob, "", 1.0, Scalar::default())?;

    // --- 2. INFERENCIA PURA (Aquí medimos) ---
    // Solo medimos el paso de los tensores por las capas de la red neuronal
    let inference_start = std::time::Instant::now();
    let mut output_blobs: Vector<Mat> = Vector::new();
    net.forward(&mut output_blobs, &net.get_unconnected_out_layers_names()?)?;
    let pure_inference_ms = inference_start.elapsed().as_secs_f64() * 1000.0;

    // --- 3. POSTPROCESAMIENTO ---
    let output = output_blobs.get(0)?;
    let size = output.mat_size();

    // 4. Parsear salida YOLOv8
    let is_yolov8 = size[1] < size[2];
    let num_preds = if is_yolov8 {
        size[2] as usize
    } else {
        size[1] as usize
    };
    let num_attrs = if is_yolov8 {
        size[1] as usize
    } else {
        size[2] as usize
    };

    let x_factor = frame_small.cols() as f32 / INPUT_SIZE as f32;
    let y_factor = frame_small.rows() as f32 / INPUT_SIZE as f32;
    let data = output.data_typed::<f32>()?;

    let mut confidences: Vector<f32> = Vector::new();
    let mut boxes: Vector<Rect> = Vector::new();
    let mut classes: Vector<i32> = Vector::new();

    for p in 0..num_preds {
        let cx = data[0 * num_preds + p];
        let cy = data[1 * num_preds + p];
        let w = data[2 * num_preds + p];
        let h = data[3 * num_preds + p];

        let num_classes = num_attrs - 4;
        let best = (0..num_classes)
            .filter(|&c| CLASES_OBJETIVO.contains(&c))
            .map(|c| (c, data[(4 + c) * num_preds + p]))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let Some((best_class, conf)) = best else {
            continue;
        };

        if conf >= CONF_THRESHOLD {
            boxes.push(Rect::new(
                ((cx - w / 2.0) * x_factor) as i32,
                ((cy - h / 2.0) * y_factor) as i32,
                (w * x_factor) as i32,
                (h * y_factor) as i32,
            ));
            confidences.push(conf);
            classes.push(best_class as i32);
        }
    }

    // 5. NMS
    let mut indices: Vector<i32> = Vector::new();
    dnn::nms_boxes(
        &boxes,
        &confidences,
        CONF_THRESHOLD,
        NMS_THRESHOLD,
        &mut indices,
        1.0,
        0,
    )?;

    // 6. Empaquetar detecciones
    let mut detections = Vec::new();
    let mut final_classes = Vec::new();

    for idx in indices {
        let rect = boxes.get(idx as usize)?;
        let conf = confidences.get(idx as usize)?;
        let cls = classes.get(idx as usize)? as usize;

        detections.push(Detection {
            bbox: [
                (rect.x as f32 * inv_scale) as f64,
                (rect.y as f32 * inv_scale) as f64,
                ((rect.x + rect.width) as f32 * inv_scale) as f64,
                ((rect.y + rect.height) as f32 * inv_scale) as f64,
            ],
            score: conf as f64,
            feat: Vec::new(),
        });
        final_classes.push(cls);
    }

    // Devolvemos la tupla con los milisegundos incluidos
    Ok((detections, final_classes, pure_inference_ms))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ROS 2 Setup
    let context = Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("realsense_mot_tracker")?;

    let (tx, rx) = mpsc::sync_channel::<Mat>(1);

    // Suscriptor a la cámara
    let _sub = node.create_subscription::<RosImage, _>(
        "/camera/camera/color/image_raw",
        move |msg: RosImage| {
            if let Ok(mat) = image_to_mat(&msg) {
                let _ = tx.try_send(mat);
            }
        },
    )?;

    // Publicador para la imagen procesada
    let publisher = node.create_publisher::<RosImage>("/vision/tracking_result")?;

    std::thread::spawn(move || {
        println!("[OK] Hilo ROS 2 escuchando en segundo plano...");
        let errors = executor.spin(SpinOptions::default());
        if !errors.is_empty() {
            eprintln!("Errores spin: {:?}", errors);
        }
    });

    // YOLO Setup
    println!("[OK] Cargando YOLOv8 desde {}...", YOLO_MODEL_PATH);
    let mut net = dnn::read_net_from_onnx(YOLO_MODEL_PATH)?;
    net.set_preferable_backend(dnn::DNN_BACKEND_OPENCV)?;
    net.set_preferable_target(dnn::DNN_TARGET_CPU)?;
    println!("[OK] YOLOv8 cargado.");

    // Tracker Setup
    let args = Args {
        max_time_lost: 30,
        det_thr: 0.3,
        match_thr: 0.8,
        penalty_p: 0.05,
        penalty_q: 0.05,
        reduce_step: 0.05,
        init_thr: 0.4,
        tai_thr: 0.5,
        min_len: 3,
    };
    let mut tracker = Tracker::new(args, "realsense_stream");
    let mut track_classes: HashMap<usize, usize> = HashMap::new();

    let mut frames_recibidos: u32 = 0;
    println!("Esperando conexión con la cámara...");

    while let Ok(frame) = rx.recv() {
        let frame_start = Instant::now();
        frames_recibidos += 1;

        // --- 1. Detección (YOLO total) ---
        let yolo_total_start = Instant::now();

        // Desestructuramos el tercer valor: la inferencia pura en ms
        let (detections, class_ids, pure_inference_ms) = match detect_yolo(&mut net, &frame) {
            Ok(tuple) => tuple,
            Err(e) => {
                eprintln!("[ERROR] YOLO: {e}");
                (vec![], vec![], 0.0)
            }
        };
        // Calculamos el tiempo total y el overhead
        let yolo_total_ms = yolo_total_start.elapsed().as_secs_f64() * 1000.0;
        let yolo_overhead_ms = yolo_total_ms - pure_inference_ms;

        // --- 2. Tracker Update ---
        let tracker_start = Instant::now();
        let tracks = tracker.update(detections.clone(), Vec::new());
        let tracker_ms = tracker_start.elapsed().as_secs_f64() * 1000.0;

        // SOLUCIÓN AL ERROR: Declaramos confirmed aquí para que el dibujo y la telemetría puedan usarlo
        let confirmed: Vec<_> = tracks
            .iter()
            .filter(|t| t.state == TrackState::Confirmed)
            .collect();

        // --- 3. Zona de dibujo ---
        let mut display_frame = frame.clone();
        let color_texto = Scalar::new(255.0, 255.0, 255.0, 0.0);

        for track in &confirmed {
            let bbox = track.x1y1wh();
            let cx = bbox[0] + bbox[2] / 2.0;
            let cy = bbox[1] + bbox[3] / 2.0;

            let mut min_dist = f64::MAX;
            let mut best_class = 0;

            for (det, cls) in detections.iter().zip(class_ids.iter()) {
                let det_cx = (det.bbox[0] + det.bbox[2]) / 2.0;
                let det_cy = (det.bbox[1] + det.bbox[3]) / 2.0;
                let dist = (cx - det_cx).hypot(cy - det_cy);

                if dist < min_dist {
                    min_dist = dist;
                    best_class = *cls;
                }
            }

            if min_dist < 50.0 {
                track_classes.insert(track.track_id, best_class);
            }

            let clase_actual = track_classes.get(&track.track_id).copied().unwrap_or(0);

            let color_caja = match clase_actual {
                0 => Scalar::new(0.0, 255.0, 0.0, 0.0),
                41 => Scalar::new(255.0, 0.0, 0.0, 0.0),
                _ => Scalar::new(0.0, 255.0, 255.0, 0.0),
            };

            let rect = Rect::new(
                bbox[0] as i32,
                bbox[1] as i32,
                bbox[2] as i32,
                bbox[3] as i32,
            );
            imgproc::rectangle(&mut display_frame, rect, color_caja, 2, imgproc::LINE_AA, 0)
                .unwrap_or(());

            let label = format!("ID:{} | C:{}", track.track_id, clase_actual);
            imgproc::put_text(
                &mut display_frame,
                &label,
                core::Point::new(rect.x, rect.y - 10),
                imgproc::FONT_HERSHEY_SIMPLEX,
                0.6,
                color_texto,
                2,
                imgproc::LINE_AA,
                false,
            )
            .unwrap_or(());
        }

        // --- 4. Publicación ---
        if let Ok(msg) = mat_to_image(&display_frame, "camera_frame") {
            publisher
                .publish(&msg)
                .unwrap_or_else(|e| eprintln!("Error publicando: {e}"));
        }

        // --- TELEMETRÍA PARA INVESTIGACIÓN ---
        let total_processing_ms = frame_start.elapsed().as_secs_f64() * 1000.0;

        if frames_recibidos % 30 == 0 {
            let fps_reales = 1000.0 / total_processing_ms;
            let otros_ms = total_processing_ms - yolo_total_ms - tracker_ms;

            println!("\n============================================================");
            println!(
                " FRAME {} | Resolución: {}x{} | Tracks activos: {}",
                frames_recibidos,
                frame.cols(),
                frame.rows(),
                confirmed.len()
            );
            println!("------------------------------------------------------------");
            println!(
                "   Rendimiento Global: {:.2} FPS ({:.2} ms/frame)",
                fps_reales, total_processing_ms
            );
            println!("------------------------------------------------------------");
            println!("  DESGLOSE DE TIEMPOS:");
            println!(
                "   • Inferencia Red Neuronal (net.forward): {:.2} ms",
                pure_inference_ms
            );
            println!(
                "   • Overhead YOLO (Resize, Blob, NMS):     {:.2} ms",
                yolo_overhead_ms
            );
            println!(
                "   • Lógica Tracker (Asignación/Kalman):    {:.2} ms",
                tracker_ms
            );
            println!(
                "   • Otros (Dibujo OpenCV, Mensajes ROS):   {:.2} ms",
                otros_ms
            );
            println!("============================================================\n");
        }
    }
    Ok(())
}
