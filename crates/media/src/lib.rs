use anyhow::Error;
use db::repositories::Repositories;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tracing::{debug, info};

fn get_codec_info(path: &Path) -> Result<String, Error> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("stream=codec_name,codec_type")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(
            path.to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid path"))?,
        )
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to get codec info"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_string())
}

fn path_has_video_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        return matches!(
            ext_str.as_str(),
            "mp4"
                | "mkv"
                | "avi"
                | "mov"
                | "flv"
                | "wmv"
                | "webm"
                | "mpeg"
                | "mpg"
                | "3gp"
                | "ts"
                | "m4v"
        );
    }
    false
}

fn get_all_video_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut video_files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        // trace!("Checking entry: {:?}", entry);
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path_has_video_extension(&path) {
            video_files.push(path);
        }
    }
    Ok(video_files)
}

#[derive(Debug, Clone)]
struct VideoFileInfo {
    path: PathBuf,
    codec_info: String,
}

pub async fn scan(dir: &Path, repositories: &Arc<Repositories>) {
    let res = get_all_video_files_in_dir(dir).unwrap_or_default();

    info!("Found {} video files", res.len());

    let mut not_cached = Vec::new();

    for path in res {
        let path_str = path.to_string_lossy().to_string();
        let path_str = path_str.as_str();
        if !repositories
            .files_repo
            .contains_path(path_str)
            .await
            .unwrap_or_else(|e| {
                tracing::log::error!("Failed to check for existing files: {}", e);
                false
            })
        {
            not_cached.push(path_str.to_string());
        }
    }

    info!("Found {} video files not cached", not_cached.len());

    let mut info = not_cached
        .par_iter()
        .map(|path| {
            let codec_info = get_codec_info(Path::new(path)).unwrap_or_else(|e| {
                tracing::log::error!("Failed to get codec info for {}: {}", path, e);
                String::new()
            });
            VideoFileInfo {
                path: PathBuf::from(path),
                codec_info,
            }
        })
        .collect::<Vec<VideoFileInfo>>();

    info!("Got codec info for {} video files", info.len());
    debug!("Codec info: {:#?}", info);

    let mut wrong_codec_only = info.clone();
    wrong_codec_only.retain(|file_info| !file_info.codec_info.contains("av1"));

    debug!("Filtered codec info: {:#?}", wrong_codec_only);

    info!("Files to convert: {}", wrong_codec_only.len());

    repositories
        .files_repo
        .add_files(
            info.iter()
                .map(|file_info| db::models::db_files::Files {
                    path: file_info.path.to_string_lossy().to_string(),
                    codec: file_info.codec_info.clone(),
                })
                .collect(),
        )
        .await
        .unwrap_or_else(|e| {
            tracing::log::error!("Failed to add files to repository: {}", e);
        });
    debug!("Added files to repository");
}
