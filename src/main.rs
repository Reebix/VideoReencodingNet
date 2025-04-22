#[macro_use]
extern crate rocket;

use lazy_static::lazy_static;
use rocket::data::{Limits, ToByteUnit};
use rocket::fs::{NamedFile, TempFile};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio;
use rocket::tokio::time::{sleep, Duration};
use serde::de::value::StrDeserializer;
use std::collections::VecDeque;
use std::fmt::format;
use std::io;
use std::io::Write;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct Task<'r> {
    description: &'r str,
    complete: bool,
}

lazy_static! {
    static ref FILE_LIST: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    static ref FILE_QUEUE: Mutex<VecDeque<PathBuf>> = Mutex::new(VecDeque::new());
    static ref SCANNED_FILES: Mutex<usize> = Mutex::new(0);
    static ref BASE_PATH: Mutex<String> = Mutex::new("".parse().unwrap());
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
    if file
        .persist_to(base_path + "/" + path.to_str().unwrap())
        .await
        .is_err()
    {
        return "Fehler beim Speichern der Datei.";
    }

    "Datei erfolgreich hochgeladen!"
}

#[get("/")]
fn base() -> String {
    format!(
        "Status:\n\
    Total Files: {}\n\
    Scanned Files: {}\n\
    Files To Convert: {}",
        FILE_LIST.lock().unwrap().len(),
        SCANNED_FILES.lock().unwrap(),
        FILE_QUEUE.lock().unwrap().len()
    )
}

#[get("/files/<file..>")]
async fn files(file: PathBuf) -> Option<NamedFile> {
    let file = Path::new(&BASE_PATH.lock().unwrap().to_string()).join(file);
    NamedFile::open(&file).await.ok()
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

fn get_ffprobe_info(path: &Path) -> String {
    let output = Command::new("ffprobe")
        .arg("-i")
        .arg(path.to_str().unwrap())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.to_string()
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

async fn scan(path: &Path) {
    // get all files in the directory
    let files = get_all_files(path);
    FILE_LIST.lock().unwrap().extend(files.clone());
    println!("Found {} files", files.len());
    let mut queue = VecDeque::new();
    // print all files
    for file in files {
        // get ffmpeg info
        let codec_info = get_codec_info(&file);
        let codec = codec_info.lines().next().unwrap();
        // check if the codec is av1
        if codec == "h264" {
            queue.push_back(file.clone());
        }
        let mut scanned_files = SCANNED_FILES.lock().unwrap();
        *scanned_files += 1;
    }
    FILE_QUEUE.lock().unwrap().extend(queue.clone());
    println!("{} of Which where in the wrong codec", queue.len());
}

#[launch]
#[tokio::main]
async fn rocket() -> _ {
    // get input from user
    let mut input = String::new();
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
    let input = input.trim();
    // check if the file exists
    let file = Path::new(input);
    if file.exists() {
        println!("File exists");
    } else {
        println!("File does not exist");
    }
    // check if the file is a directory
    if file.is_dir() {
        println!("File is a directory");
    } else {
        println!("File is not a directory");
    }

    let mut bp = BASE_PATH.lock().unwrap();
    *bp = file.to_str().unwrap().to_string();

    let file_clone = file.to_path_buf();
    tokio::spawn(async move {
        // scan the directory for files
        scan(&file_clone).await;
    });

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
}
