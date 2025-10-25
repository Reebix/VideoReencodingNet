#[macro_use]
extern crate rocket;

use clap::Parser;
use lazy_static::lazy_static;
use rocket::data::{Limits, ToByteUnit};
use rocket::fs::{NamedFile, TempFile};
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

const CACHE_FILE: &str = "cache.json";

#[derive(Serialize, Deserialize)]
struct Task<'r> {
    description: &'r str,
    complete: bool,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Path to scan for video files
    #[arg(short, long, default_value_t = String::from(""))]
    path: String,
    /// Clean cache before scanning
    #[arg(short, long, default_value_t = false)]
    clean: bool,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    file_list: Vec<PathBuf>,
}

lazy_static! {
    static ref FILE_LIST: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    static ref FILE_QUEUE: Mutex<VecDeque<PathBuf>> = Mutex::new(VecDeque::new());
    static ref SCANNED_FILES: Mutex<usize> = Mutex::new(0);
    static ref BASE_PATH: Mutex<String> = Mutex::new("".parse().unwrap());
    static ref TOTAL_SIZE: Mutex<f64> = Mutex::new(0.0);
    static ref CACHE: Mutex<Cache> = Mutex::new(Cache {
        file_list: Vec::new(),
    });
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
        Files To Convert: {}\n\
        Total Size To Convert: {} MB",
        FILE_LIST.lock().unwrap().len(),
        SCANNED_FILES.lock().unwrap(),
        FILE_QUEUE.lock().unwrap().len(),
        TOTAL_SIZE.lock().unwrap()
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
            if path.is_file() && path.extension().is_some_and(|ext| ext == "mp4") {
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

fn clear_queue() {
    let mut fq = FILE_QUEUE.lock().unwrap();
    *fq = VecDeque::new();
    let mut sf = SCANNED_FILES.lock().unwrap();
    *sf = 0;
    let mut fl = FILE_LIST.lock().unwrap();
    *fl = Vec::new();
    println!("Cleared queue");
}

fn file_is_video(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        let ext = ext.to_str().unwrap().to_lowercase();
        ext == "mp4" || ext == "mkv" || ext == "avi" || ext == "mov" || ext == "wmv"
    })
}

fn file_in_cache(path: &Path) -> bool {
    let cache = CACHE.lock().unwrap();
    cache.file_list.contains(&path.to_path_buf())
}

async fn scan(path: &Path) {
    let mut file_size = 0;
    clear_queue();
    // get all files in the directory
    let files = get_all_files(path);
    let mut file_copy: Vec<PathBuf> = Vec::new();
    for file in files {
        if !file_is_video(&file) {
            continue;
        }
        file_copy.push(file);
    }

    FILE_LIST.lock().unwrap().extend(file_copy.clone());
    println!("Found {} files", file_copy.len());
    // print all files
    for file in file_copy {
        // get ffmpeg info
        let codec_info = get_codec_info(&file);
        let codec = codec_info.lines().next().unwrap();
        // check if the codec is not av1
        if codec != "av1" && !file_in_cache(&file) {
            FILE_QUEUE.lock().unwrap().push_back(file.clone());
        } else if codec == "av1" {
            CACHE.lock().unwrap().file_list.push(file.clone());
        }

        file_size += file.metadata().unwrap().len();
        let mut scanned_files = SCANNED_FILES.lock().unwrap();
        *scanned_files += 1;
    }
    println!(
        "{} of which where in the wrong codec",
        FILE_QUEUE.lock().unwrap().len()
    );

    // format size of files
    let file_size = file_size as f64 / 1_000_000.0;
    let file_size = format!("{:.2}", file_size);
    let mut total_size = TOTAL_SIZE.lock().unwrap();
    *total_size = file_size.parse().unwrap();
    println!("Total size of files: {} MB", file_size);
}

fn init_cache(clean: bool) {
    if clean {
        println!("Cleaning cache...");
        return;
    }
    let cache_path = Path::new(CACHE_FILE);
    if cache_path.exists() {
        let cache_data = std::fs::read_to_string(cache_path).unwrap();
        let cache: Cache = serde_json::from_str(&cache_data).unwrap();
        let mut file_list = FILE_LIST.lock().unwrap();
        *file_list = cache.file_list;
        println!("Loaded cache with {} files", file_list.len());
    } else {
        println!("No cache found, starting fresh");
    }
}

fn save_cache() {
    let cache_path = Path::new(CACHE_FILE);
    let cache_lock = CACHE.lock().unwrap();
    let cache_data = serde_json::to_string(&*cache_lock).unwrap();
    std::fs::write(cache_path, cache_data).unwrap();
    println!("Cache saved with {} files", cache_lock.file_list.len());
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    init_cache(args.clean);

    // get input from user
    let mut _input = String::new();
    if !args.path.is_empty() {
        _input = args.path;
        println!("base_path: {}", _input);
    } else {
        #[cfg(not(debug_assertions))]
        {
            println!("Enter the path where the server should look for files: ");
            std::io::stdin()
                .read_line(&mut _input)
                .expect("Failed to read line");
        }
        #[cfg(debug_assertions)]
        {
            println!("Debug mode: using default file path");
            _input = "C:\\Users\\Rebix\\Downloads\\testcompressions".to_string()
        }

        _input = _input.trim().to_string();
    }
    scan_files(_input.to_string()).await;

    // start thread that saves config when all files are scanned
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            let scanned_files = SCANNED_FILES.lock().unwrap();
            let total_files = FILE_LIST.lock().unwrap().len();
            if *scanned_files >= total_files && total_files > 0 {
                save_cache();
                break;
            }
        }
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
        .mount("/", routes![scan_files])
        .launch()
        .await
        .expect("TODO: panic message");
}
