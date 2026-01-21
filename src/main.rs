use chrono::Local;
use clap::Parser;
use log::info;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

mod background_works;
mod image_worker;
mod loaders;
mod save;
mod vectorize;

const BASE_PATH: &str = "/Users/kapustindmitri/RustroverProjects/logoLoader/";
const JSON_FILE_PATH: &str = "export_logo_20.01.26.json";
const DOWNLOAD_FOLDER: &str = "Logo/Raw";
const UPSCALE_FOLDER: &str = "Logo/Upscale";
const LOG_FILE: &str = "logo.log";
const RESULT_FOLDER: &str = "Logo/Result";
const DOWNLOAD: bool = false;
const UPSCALE: bool = false;

#[derive(Debug, Deserialize, Clone)]
struct LogoJob {
    url: String,
    id: u32,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// JSON file with logos job
    #[arg(short, long, default_value_t = JSON_FILE_PATH.to_string())]
    job: String,

    /// PNG optimization level
    #[arg(short, long, default_value_t = BASE_PATH.to_string())]
    out_dir: String,

    #[arg(long, default_value_t = DOWNLOAD)]
    download: bool,

    /// Whether to upscale images
    #[arg(long, default_value_t = UPSCALE)]
    upscale: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let job_path = Path::new(&args.job);
    let out_dir_path = Path::new(&args.out_dir);

    // Инициализация лога
    println!("Инициализация лога");
    let _ = setup_logger(&Path::new(&args.out_dir).join(LOG_FILE));

    // Скачка задания
    println!("Скачка задания {}", job_path.to_str().unwrap());
    // let logos = load_job(JSON_FILE_PATH)?;
    let logos = loaders::load_json_job("test_save.json")?;

    // Создаем папки одним вызовом для каждой
    info!(
        "Создаем папки одним вызовом для каждой: {}",
        out_dir_path.to_str().unwrap()
    );
    for folder in &[DOWNLOAD_FOLDER, UPSCALE_FOLDER, RESULT_FOLDER] {
        create_dir(*folder)?;
    }

    // Скачка файлов задания
    if args.download {
        download_images(logos.clone()).await;
    }

    // Увеличение разрешения файлов
    if args.upscale {
        upscale_images().await?;
    }

    // Обработка файлов
    image_worker::images_works_parallel(logos).await?;

    Ok(())
}

// Создать директорию если ее не существует
fn create_dir(dir: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !Path::new(dir).exists() {
        fs::create_dir_all(dir)?;
        info!("Создана директория: {}", dir);
    }
    Ok(())
}

// Скачать все изображения с сервера
async fn download_images(logos: Vec<LogoJob>) {
    const FILE_EXTENSION: &str = ".jpg";

    // Запускаем все загрузки параллельно
    let mut tasks = Vec::new();
    let mut counter = 0;

    for logo in logos {
        tasks.push(tokio::spawn(async move {
            let filename = format!("{}/{}{}", DOWNLOAD_FOLDER, logo.id, FILE_EXTENSION);
            let _ = get_image_by_job(&logo.url, &filename).await;
            info!(
                "{} Файл '{}' -> {} успешно скачан",
                counter, logo.url, filename
            );
        }));
        counter += 1;
    }

    for task in tasks {
        let _ = task.await;
    }
}

async fn get_image_by_job(url: &str, out: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client_new = reqwest::Client::new();
    let response = client_new.get(url).send().await?;

    if response.status().is_success() {
        let bytes = response.bytes().await?;
        let mut file = File::create(out).await?;
        file.write_all(&bytes).await?;
    } else {
        println!(
            "Ошибка загрузки '{}'. Код статуса: {}",
            url,
            response.status()
        );
    }

    Ok(())
}

async fn upscale_images() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Usage: upscayl-bin -i infile -o outfile [options]...
    //
    //     -h                   show this help
    //     -i input-path        input image path (jpg/png/webp) or directory
    //     -o output-path       output image path (jpg/png/webp) or directory
    //     -z model-scale       scale according to the model (can be 2, 3, 4. default=4)
    //     -s output-scale      custom output scale (can be 2, 3, 4. default=4)
    //     -r resize            resize output to dimension (default=WxH:default), use '-r help' for more details
    //     -w width             resize output to a width (default=W:default), use '-r help' for more details
    //     -c compress          compression of the output image, default 0 and varies to 100
    //     -t tile-size         tile size (>=32/0=auto, default=0) can be 0,0,0 for multi-gpu
    //     -m model-path        folder path to the pre-trained models. default=models
    //     -n model-name        model name (default=realesrgan-x4plus, can be realesr-animevideov3 | realesrgan-x4plus-anime | realesrnet-x4plus or any other model)
    //     -g gpu-id            gpu device to use (default=auto) can be 0,1,2 for multi-gpu
    //     -j load:proc:save    thread count for load/proc/save (default=1:2:2) can be 1:2,2,2:2 for multi-gpu
    //     -x                   enable tta mode
    //     -f format            output image format (jpg/png/webp, default=ext/png)
    //     -v                   verbose output

    const BASE_PATH: &str = "/Users/kapustindmitri/RustroverProjects/logoLoader/";
    let input_path = Path::new(BASE_PATH).join(DOWNLOAD_FOLDER);
    let output_path = Path::new(BASE_PATH).join(UPSCALE_FOLDER);

    const UPSCALER_PROG: &str = "/Applications/Upscayl.app/Contents/Resources/bin/upscayl-bin";
    const MODEL_PATH: &str = "/Applications/Upscayl.app/Contents/Resources/models";
    const MODEL_NAME: &str = "upscayl-standard-4x";
    const SCALE: usize = 4;
    const COMPRESSION: usize = 100;
    const TYPE: &str = "png";

    let status = Command::new(UPSCALER_PROG)
        .arg("-i")
        .arg(input_path.to_str().expect("Invalid UTF-8 in input path"))
        .arg("-o")
        .arg(output_path.to_str().expect("Invalid UTF-8 in input path"))
        .arg("-m")
        .arg(&MODEL_PATH)
        .arg("-n")
        .arg(&MODEL_NAME)
        .arg("-s")
        .arg(SCALE.to_string())
        .arg("-f")
        .arg(&TYPE)
        .arg("-v")
        .arg("-c")
        .arg(COMPRESSION.to_string())
        .status()
        .expect("failed to execute process");

    if !status.success() {
        return Err(format!("Upscayl failed for {}", input_path.to_str().expect("err")).into());
    }

    info!("✅ Completed: {}", output_path.to_str().expect("err"));

    Ok(())
}

fn setup_logger(log_file: &Path) -> Result<(), Box<dyn Error>> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(fern::log_file(log_file)?)
        .apply()?;
    Ok(())
}
