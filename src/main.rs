#[macro_use]
extern crate rocket;

use lazy_static::lazy_static;
use rocket::fs::NamedFile;
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
}

#[post("/todo", data = "<task>")]
fn new(task: Json<Task<'_>>) {
    println!("New task: {} - {}", task.description, task.complete);
}

#[get("/delay/<seconds>")]
async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}

#[get("/hello/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
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
    println!("{} of Which where in the wron codec", queue.len());
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
        input = "X:\\anime\\Frieren".to_string()
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

    let file_clone = file.to_path_buf();
    tokio::spawn(async move {
        // scan the directory for files
        scan(&file_clone).await;
    });

    // execute ffmpeg command
    /*
     let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(file.to_str().unwrap())
         .arg("-c:v")
        .arg("av1_nvenc")
        .arg("-preset")
        .arg("p4")
         .arg("-cq")
        .arg("40")
         .arg(outfile.to_str().unwrap())
        .output()
        .expect("Failed to execute command");

    println!("status: {}", output.status);
    io::stdout().write_all(&output.stdout).expect("TODO: panic message");
    io::stderr().write_all(&output.stderr).expect("TODO: panic message");

     */
    rocket::build()
        .configure(
            rocket::Config::figment()
                .merge(("port", 8000))
                .merge(("address", "0.0.0.0")),
        )
        .mount("/", routes![hello])
        .mount("/", routes![base])
        .mount("/", routes![delay])
        .mount("/", routes![new])
}
