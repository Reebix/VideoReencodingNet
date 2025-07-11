#[macro_use]
extern crate rocket;

use clap::Parser;
use lazy_static::lazy_static;
use rocket::data::{Limits, ToByteUnit};
use rocket::fs::{NamedFile, TempFile};
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio;
use rocket::yansi::Paint;
use serde::de::value::StrDeserializer;
use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct Task<'r> {
    description: &'r str,
    complete: bool,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    #[arg(short, long, default_value_t = String::from(""))]
    path: String,
}

lazy_static! {
    static ref FILE_LIST: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    static ref FILE_QUEUE: Mutex<VecDeque<PathBuf>> = Mutex::new(VecDeque::new());
    static ref SCANNED_FILES: Mutex<usize> = Mutex::new(0);
    static ref BASE_PATH: Mutex<String> = Mutex::new("".parse().unwrap());
    static ref VIDEO_LENGTH: Mutex<f64> = Mutex::new(0f64);
}

#[get("/request")]
fn request() -> String {
    let mut file_queue = FILE_QUEUE.lock().unwrap();

    if file_queue.is_empty() {
        return "".to_string();
    }

    let mut path = file_queue
        .pop_front()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
        .replace(BASE_PATH.lock().unwrap().as_str(), "");
    path.remove(0);

    path
}

#[post("/converted/<path..>", data = "<file>")]
async fn converted(path: PathBuf, mut file: TempFile<'_>) -> &'static str {
    let base_path = BASE_PATH.lock().unwrap().to_string();
    let file_name = path
        .to_str()
        .unwrap()
        .replace("\\", "/")
        .split('/')
        .next_back()
        .unwrap()
        .to_string();
    let result = file.persist_to(format!("./{file_name}")).await;
    if result.is_err() {
        println!("Error saving file: {:?}", result.err());
        return "Fehler beim Speichern der Datei.";
    }
    std::fs::copy(
        format!("./{file_name}"),
        format!("{base_path}/{}", path.to_str().unwrap()),
    )
    .unwrap();
    std::fs::remove_file(format!("./{file_name}")).unwrap();

    "Datei erfolgreich hochgeladen!"
}

#[get("/")]
fn base() -> String {
    format!(
        "Status:\n\
    Total Files: {}\n\
    Scanned Files: {}\n\
    Files To Convert: {}",
        // Total Length: {:.2}min",
        FILE_LIST.lock().unwrap().len(),
        SCANNED_FILES.lock().unwrap(),
        FILE_QUEUE.lock().unwrap().len(),
        // VIDEO_LENGTH.lock().unwrap()
    )
}

#[get("/files/<file..>")]
async fn files(file: PathBuf) -> Option<NamedFile> {
    let file = Path::new(&BASE_PATH.lock().unwrap().to_string()).join(file);
    NamedFile::open(&file).await.ok()
}

#[post("/scan", data = "<path>", format = "text/plain")]
async fn scan_files(path: String) -> String {
    println!("Scanning file: {:?}", path);
    let mut bp = BASE_PATH.lock().unwrap();
    *bp = path;
    let base_path = bp.clone();
    let path = Path::new(&base_path);
    if path.exists() {
        tokio::spawn(async move {
            scan(Path::new(&base_path)).await;
        });
        return "Scannen gestartet".to_string();
    }
    "Datei existiert nicht".to_string()
}

fn get_all_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                files.extend(get_all_files(&path));
            }
        }
    }
    files
}

fn get_codec_info(path: &Path) -> String {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("stream=codec_name,codec_type")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path.to_str().unwrap())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.to_string()
}

fn get_video_length(path: &Path) -> String {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path.to_str().unwrap())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.to_string()
}

fn clear_queue() {
    let mut fq = FILE_QUEUE.lock().unwrap();
    *fq = VecDeque::new();
    let mut sf = SCANNED_FILES.lock().unwrap();
    *sf = 0;
    let mut fl = FILE_LIST.lock().unwrap();
    *fl = Vec::new();
    println!("Cleared queue");
}

async fn scan(path: &Path) {
    let mut file_size = 0;
    let mut length: f64 = 0f64;
    clear_queue();
    // get all files in the directory
    let files = get_all_files(path);
    FILE_LIST.lock().unwrap().extend(files.clone());
    println!("Found {} files", files.len());
    // print all files
    for file in files {
        // get ffmpeg info
        let codec_info = get_codec_info(&file);
        let codec = codec_info.lines().next().unwrap();
        // check if the codec is h264
        if codec == "h264" {
            FILE_QUEUE.lock().unwrap().push_back(file.clone());
        }

        file_size += file.metadata().unwrap().len();
        let mut scanned_files = SCANNED_FILES.lock().unwrap();
        *scanned_files += 1;

        // tokio::spawn(async move {
        //     add_file_length(&file).await;
        //     let mut scanned_files = SCANNED_FILES.lock().unwrap();
        //     *scanned_files += 1;
        // });
    }
    println!(
        "{} of Which where in the wrong codec",
        FILE_QUEUE.lock().unwrap().len()
    );

    // format size of files
    let file_size = file_size as f64 / 1_000_000.0;
    let file_size = format!("{:.2}", file_size);
    println!("Total size of files: {} MB", file_size);
}

async fn add_file_length(path: &Path) {
    let length_info = get_video_length(path);
    let length_info = length_info.trim();
    let length_info = length_info.parse::<f64>().unwrap();
    let length_info = length_info / 60.0;
    let length_info = format!("{:.2}", length_info);
    *VIDEO_LENGTH.lock().unwrap() += length_info.parse::<f64>().unwrap();
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    // print current user
    // get input from user
    let mut input = String::new();
    if !args.path.is_empty() {
        input = args.path;
        println!("base_path: {}", input);
    } else {
        #[cfg(not(debug_assertions))]
        {
            println!("Enter the path where the server should look for files: ");
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
        }
        #[cfg(debug_assertions)]
        {
            println!("Debug mode: using default file path");
            input = "C:\\Users\\Rebix\\Downloads\\testcompressions".to_string()
        }

        input = input.trim().parse().unwrap();
    }
    scan_files(input.to_string()).await;

    rocket::build()
        .configure(
            rocket::Config::figment()
                .merge(("port", 8000))
                .merge(("address", "0.0.0.0"))
                .merge(("limits", Limits::new().limit("file", 10.gigabytes()))),
        )
        .mount("/", routes![base])
        .mount("/", routes![request])
        .mount("/", routes![files])
        .mount("/", routes![converted])
        .mount("/", routes![scan_files])
        .launch()
        .await
        .expect("TODO: panic message");
}
