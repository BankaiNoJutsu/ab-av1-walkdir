use clap::{Parser};
use std::{vec::Vec, string::String, env};
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};
use std::path::Path;
use colored::Colorize;
use clearscreen::clear;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Serialize, Deserialize, Debug)]
#[clap(name = "ab-av1-walkdir",
       author = "BankaiNoJutsu <lbegert@gmail.com>",
       about = "ab-av1 walkdir, process all files in a given folder",
       long_about = None)]

struct Args {
    // Path to the folder containing the video files
    #[clap(short, long)]
    folder: String,
    // min vmaf
    #[clap(short, long, default_value = "95")]
    vmaf: i8,
    // encoder
    #[clap(short, long, default_value = "libx265")]
    encoder: String,
}

fn main() {
    let args = Args::parse();
    let folder = args.folder;
    let vmaf = args.vmaf;
    let encoder = args.encoder;
    let codec = "x265".to_string();

    match encoder.as_str() {
        "libx265" => {
            let _codec = "x265".to_string();
        }
        "av1" => {
            let _codec = "av1".to_string();
        }
        _ => {
            println!("{} is not a valid encoder!", encoder);
            std::process::exit(1);
        }
    }

    // if folder is not a folder, exit
    if !Path::new(&folder).is_dir() {
        println!("{} is not a folder!", folder);
        std::process::exit(1);
    }

    let files = walk_files(&folder);
    let to_process = files.clone();

    // print the number of files found in the folder
    println!("Found {} files in folder!", files.len());

    // progressbar setup
    // TODO: fix the count of files in the progress bar
    let bar = ProgressBar::new(files.len() as u64);
    let bar_style = "[file_count][{elapsed_precise}] [{wide_bar:.green/white}] {percent}% {pos:>7}/{len:7} files       eta: {eta:<7}";
    bar.set_style(
        ProgressStyle::default_bar()
            .template(bar_style)
            .unwrap()
            .progress_chars("#>-"),
    );

    let result = clear();
    match result {
        Ok(_) => {}
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    // if binary 'ab-av1' is not in the path, exit
    if !std::path::Path::new("ab-av1.exe").exists() {
        println!("Binary 'ab-av1.exe' not found in current path!");
        println!("Searching for ab-av1.exe in system path...");
        // search for binary in system path
        let output = std::process::Command::new("where")
            .arg("ab-av1.exe")
            .output()
            .expect("failed to execute process");
        println!("ab-av1.exe found in: {}", String::from_utf8_lossy(&output.stdout));
        if output.stdout.is_empty() {
            println!("ab-av1.exe not found in system path!");
            std::process::exit(1);
        } else {
            // run ab-av1 for each file in the folder, with the given vmaf and encoder
            process_sequential(to_process, vmaf, encoder, bar, codec);
        }
    } else {
        // run ab-av1 for each file in the folder, with the given vmaf and encoder
        process_sequential(to_process, vmaf, encoder, bar, codec);
    }
}

// function to process all files in a given folder, but wait for each process to finish before starting the next one (sequential)
pub fn process_sequential(mut files: Vec<String>, vmaf: i8, encoder: String, bar: ProgressBar, codec: String) {
    let mut removed_files = Vec::new();
    for file in &files {
        if file.contains(&codec) {
            let stem = Path::new(&file).file_stem().unwrap().to_str().unwrap().to_string();
            let extension = Path::new(&file).extension().unwrap().to_str().unwrap().to_string();
            let directory = Path::new(&file).parent().unwrap().to_str().unwrap().to_string();
            removed_files.push(format!("{}\\{}.{}", directory, stem.replace(format!(".{}", codec).as_str(), ""), extension));
            removed_files.push(file.to_string());
            println!("{}", format!("{}\\{}.{}", directory, stem.replace(format!(".{}", codec).as_str(), ""), extension));
        }
        if file.contains("sample") {
            removed_files.push(file.to_string());
        }
        // if file size is less than 400MB, remove it from the files vector
        if Path::new(&file).metadata().unwrap().len() < 400000000 {
            removed_files.push(file.to_string());
        }
    }
    // keep only files that are not in the removed_files vector
    files.retain(|x| !removed_files.contains(x));

    // set progress bar to the number of files to process
    bar.set_length(files.len() as u64);

    // set progress bar to 0
    bar.set_position(0);

    for file in files {
        let result = clear();
        match result {
            Ok(_) => {}
            Err(e) => {
                println!("Error: {}", e);
            }
        }
        bar.inc(1);
        let mut output = std::process::Command::new("ab-av1.exe")
            .arg("auto-encode")
            .arg("-i")
            .arg(&file)
            .arg("--min-vmaf")
            .arg(vmaf.to_string())
            .arg("--acodec")
            .arg("aac")
            .arg("--downmix-to-stereo")
            .arg("-e")
            .arg(&encoder)
            .spawn()
            .expect("failed to execute process");
        output.wait().expect("failed to wait on child");
        if output.wait().unwrap().success() {
            println!("{}",format!("{} was encoded successfully with VMAF of {}!", file, vmaf).green());
            let result = clear();
            match result {
                Ok(_) => {}
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        } else {
            // if ab-av1.exe fails with the error containing "Error: Failed to find a suitable crf", lower the vmaf by 1 and try again in a while loop
            '_inner: while output.wait().unwrap().success() == false {
                for vmaf_dec in (1..vmaf).rev().step_by(1) {
                    // if ab-av1.exe fails with the error containing "Error: Failed to find a suitable crf", lower the vmaf by 1 and try again in a while loop
                    let mut output = std::process::Command::new("ab-av1.exe")
                        .arg("auto-encode")
                        .arg("-i")
                        .arg(&file)
                        .arg("--min-vmaf")
                        .arg((vmaf_dec).to_string())
                        .arg("-e")
                        .arg(&encoder)
                        .spawn()
                        .expect("failed to execute process");
                    output.wait().expect("failed to wait on child");
                    if output.wait().unwrap().success() {
                        println!("{}",format!("{} was encoded successfully with VMAF of {}!", file, vmaf_dec).green());
                        break '_inner;
                    } else {
                        println!("{}",format!("{} was not encoded successfully with VMAF of {}! Retrying with VMAF of {}...", file, vmaf_dec, vmaf_dec-1).red());
                        continue;
                    }
                }
            }
        }
    }
}

pub fn walk_count(dir: &String) -> usize {
    let mut count = 0;
    for e in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if e.metadata().unwrap().is_file() {
            let filepath = e.path().display();
            let str_filepath = filepath.to_string();
            let mime = find_mimetype(&str_filepath);
            if mime.to_string() == "VIDEO" {
                count = count+1;
            }
        }
    }
    println!("Found {} valid video files in folder!", count);
    return count;
}

pub fn walk_files(dir: &String) -> Vec<String>{
    let mut arr = vec![];
    let mut index = 0;

    for e in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if e.metadata().unwrap().is_file() {
            let filepath = e.path().display();
            let str_filepath = filepath.to_string();
            let mime = find_mimetype(&str_filepath);
            if mime.to_string() == "VIDEO" {
                let file = absolute_path(&e.path().display().to_string());
                arr.insert(index, file);
                index = index + 1;
            }
        }
    }
    return arr;
}

pub fn find_mimetype(filename :&String) -> String{

    let parts : Vec<&str> = filename.split('.').collect();

    let res = match parts.last() {
            Some(v) =>
                match *v {
                    "mkv" => "VIDEO",
                    "avi" => "VIDEO",
                    "mp4" => "VIDEO",
                    "divx" => "VIDEO",
                    "flv" => "VIDEO",
                    "m4v" => "VIDEO",
                    "mov" => "VIDEO",
                    "ogv" => "VIDEO",
                    "ts" => "VIDEO",
                    "webm" => "VIDEO",
                    "wmv" => "VIDEO",
                    &_ => "OTHER",
                },
            None => "OTHER",
        };
    return res.to_string();
}

pub fn absolute_path(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .expect("could not get current path")
            .join(path)
    };

    absolute_path.into_os_string().into_string().unwrap()
}