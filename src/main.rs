extern crate bytesize;

use fltk::app::quit;
use fltk::{app, button::Button, frame::Frame, prelude::*, window::Window};
use clap::{Arg, arg, ArgAction, Command};
use bytesize::ByteSize;
use serde_derive::Deserialize;
use time::OffsetDateTime;
use unrar::{ListSplit, VolumeInfo};
use zip::{CompressionMethod, DateTime};
use std::{env, ffi::OsStr, fs, time::SystemTime, path::PathBuf};
use std::fs::File;
use std::io::BufReader;


#[derive(Deserialize, Debug)]
struct Extension {
    extension: String,
    name: String,
    description: String,
    further_reading: String,
    preferred_mime: String,
    mime: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct ExtensionVec {
    extensions: Vec<Extension>,
}

struct Arguments {
    file_path: PathBuf,
    extensions_path: PathBuf,
    is_debug: bool,
    is_human: bool,
    only_general: bool,
    ignore_general: bool,
    extension_info: bool
}

fn get_general_info(args: &Arguments){
    println!("## General information:");
    println!("# Name: {}",args.file_path.file_name().unwrap().to_string_lossy());
    
    let metadata = fs::metadata(args.file_path.clone()).unwrap();

    if !args.is_human {println!("# Size: {:?}", metadata.len())}
    else { println!("# Size: {}",ByteSize(metadata.len()).to_string_as(true));}
    // TODO: proper handling of inaccessible time
    let created_time : OffsetDateTime = metadata.created().unwrap_or(SystemTime::now()).into();
    let modified_time : OffsetDateTime = metadata.modified().unwrap_or(SystemTime::now()).into();
    let accessed_time : OffsetDateTime = metadata.accessed().unwrap_or(SystemTime::now()).into();
    
    println!("# Created: {:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}", created_time.year(), created_time.month() as u8, created_time.day(), created_time.hour(), created_time.minute(), created_time.second());
    println!("# Last modified: {:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}", modified_time.year(), modified_time.month() as u8, modified_time.day(), modified_time.hour(), modified_time.minute(), modified_time.second());
    println!("# Last accessed: {:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2}", accessed_time.year(), accessed_time.month() as u8, accessed_time.day(), accessed_time.hour(), accessed_time.minute(), accessed_time.second());
    
    if metadata.permissions().readonly() { println!("Readonly");} 
    else { println!("# Readable and writable");}
}

/// Contain extension data as global to minimize calls --- TODO URGENT
fn get_extension_name(args: &Arguments, extension: &OsStr) -> String {
    let extensions_str = match fs::read_to_string(args.extensions_path.clone()) {
        Ok(c) => c,
        Err(_) => {
            println!("Could not read extensions file: {}", args.extensions_path.to_string_lossy());
            quit();
            unreachable!();
        }
    };

    let mut extension_vec: ExtensionVec = toml::from_str(&extensions_str).unwrap();
    for extension_data in extension_vec.extensions.iter_mut() {
        if extension_data.extension != extension.to_str().unwrap() {continue};
        return extension_data.name.clone();
    }
    "unknown type".to_string()
}

fn get_extension_info(extension: &str, more_info: bool, extensions_path: &PathBuf) {
    println!("## Extension: {}", extension);
    let extensions_str = match fs::read_to_string(extensions_path) {
        Ok(c) => c,
        Err(_) => {
            println!("Could not read extensions file: {}", extensions_path.to_string_lossy());
            return;
        }
    };

    let mut extension_vec: ExtensionVec = toml::from_str(&extensions_str).unwrap();
    for extension_data in extension_vec.extensions.iter_mut() {
        if extension_data.extension.ne(extension) {continue};
        //TODO: make a switch either two or more same extensions with different data types found in that file
        println!("# Name: {}", extension_data.name);
        println!("# Media type (mime): {}", extension_data.preferred_mime);
    
        if more_info {
            if extension_data.mime.len() > 1{
                print!("# Other possible media types (mimes): ");
                for mime in extension_data.mime.iter_mut() {
                    if mime == &extension_data.preferred_mime { continue;}
                    print!("{}; ", mime);
                }
                println!();
            }
            println!("# Description: {}", extension_data.description);
            println!("# Further reading: {}", extension_data.further_reading)
        }
    }
}

fn get_rar_info(args: &Arguments) {
    println!("## RAR information");
    let mut option = None;
    match unrar::Archive::new(&args.file_path).break_open::<ListSplit>(Some(
        &mut option
    )) {
        // Looks like I need to write my own implementations of rar lib
        Ok(archive) => {
            if archive.has_comment() { println!("# Comment: currently not supported", )}
            if archive.volume_info() != VolumeInfo::None {
                println!("# This is multi-part archive, it is not supported for now.");
                return;
            }
            if let Some(error) = option {
                // If the error's data field holds an OpenArchive, an error occurred while opening,
                // the archive is partly broken (e.g. broken header), but is still readable from.
                // In this example, we are still going to use the archive and list its contents.
                println!("Error: {}, continuing.", error);
            }
            for entry in archive {
                match entry {
                    Ok(e) => println!("{}", e),
                    Err(err) => println!("Error: {}", err),
                }
            }    
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

fn get_zip_info(args: &Arguments, buf_reader: BufReader<File>) {
    println!("## ZIP information");
    let mut archive = zip::ZipArchive::new(buf_reader).unwrap();
    if !archive.comment().is_empty() {
        println!("# Comment: {:?}", std::str::from_utf8(archive.comment()).unwrap());
    }

    print!("# Compressed size: ");
    let size : u64 = fs::metadata(args.file_path.clone()).unwrap().len();
    let decompressed_size : u64 = archive.decompressed_size().unwrap_or(0).try_into().unwrap();
    let mut percent = (size as f32 / decompressed_size as f32)*100.;
    if percent > 100. { percent = 100.}
    if args.is_human { println!("{}/{} ({:.2}%)", 
        ByteSize(size).to_string_as(true),
        ByteSize(decompressed_size).to_string_as(true),
        percent
    )}
    else { println!("{}/{} ({:.2}%)",size, decompressed_size, percent) ; }
    
    // While we gather zip file information, gather also used compression methods
    let mut compression_methods : Vec<CompressionMethod> = Vec::new();
    println!("# Zip file contains:");
    for i in 0..archive.len() {
        let file = match archive.by_index(i) {
            Ok(file) => file,
            Err(e) => {
                println!("Error (most likely encrypted file): {}", e);
                continue;
            }
        };
        
        if !compression_methods.contains(&file.compression()) {
            compression_methods.push(file.compression());
        }

        let outpath = match file.enclosed_name() {
            Some(path) => path,
            None => {
                println!("File {} has a suspicious path", file.name());
                continue;
            }
        };
        
        // Comment scope
        {
            let comment = file.comment();
            let name = file.name();
            if !comment.is_empty() {
                println!("File {name} has comment: {comment}");
            }
        }

        if file.is_dir() {
            println!(
                "\"{}\"",
                outpath.display()
            );
        } else {
            let last_modified : DateTime = file.last_modified().unwrap_or_default();
            let percent = format!("{:.prec$}", ((file.compressed_size() as f32 / file.size() as f32) * 100.).min(100.), prec = 2) .to_string();
            let file_size : String = if args.is_human {
                ByteSize(file.compressed_size()).to_string_as(true) +
                "/" +
                &ByteSize(file.size()).to_string_as(true) +
                ") (" +
                &percent +
                "%"
            }
            else {
                file.compressed_size().to_string() +
                "/" +
                &file.size().to_string() +
                ") (" +
                &percent +
                "%"
            };
            print!(
                "\"{}\" ({}) ({}) (last modified: {}) ({})",
                outpath.display(),
                file_size,
                get_extension_name(args, file.mangled_name().extension().unwrap_or(OsStr::new(""))),
                last_modified,
                file.crc32()
            );
            
            // Unreachable for now
            if file.encrypted() {
                print!(" (encrypted)");
            }
            println!();
        }
    }
    print!("# Compression methods used: ");
    for method in compression_methods.iter_mut() {
        print!("{} ", method);
    }
    println!()
}

fn get_info(args: &Arguments) {
    if !args.file_path.exists() { println!("Path to file does not exist.");}
    if !args.file_path.is_file() { println!("Path to file leads to directory, not file");}

    let file_extension: &std::ffi::OsStr = args.file_path.extension().unwrap_or(OsStr::new(""));
    let buf_reader : BufReader<fs::File> = BufReader::new(fs::File::open(&args.file_path).unwrap());
    
    if !args.ignore_general {get_general_info(args)};
    get_extension_info(file_extension.to_str().unwrap_or(""), args.extension_info, &args.extensions_path);
    // Specific use-cases
    if !args.only_general { 
        if file_extension.eq("zip") { get_zip_info(args, buf_reader) }
        else if file_extension.eq("rar") { get_rar_info(args) };
    }
}

fn main() {
    // Console arguments
    let m = Command::new("fat-rs")
        .author("caffidev, caffidev@gmail.com")
        .version("0.1.1")
        .about("fat-rs - File Analysis Tool, analyzes metadata and tries to guess its extension.")
        .disable_help_subcommand(true)
        .disable_help_flag(true)
        .arg(arg!(<FILE> ... "File to analyze").value_parser(clap::value_parser!(PathBuf)))
        .arg(
            Arg::new("help")
                .short('?')
                .long("help")
                .action(ArgAction::Help)
                .help("Prints help (this message).")
        )
        .arg(
            Arg::new("extension-info")
            .action(ArgAction::SetTrue)
                .short('e')
                .long("extension-info")
                .help("Provides more info about extension: MIME type, where to read about it etc..")
        )
        .arg(
            Arg::new("debug")
                .action(ArgAction::SetTrue)
                .short('d')
                .long("debug")
                .help("Turns on debugging mode.")
        )
        .arg(
            Arg::new("human")
            .action(ArgAction::SetTrue)
                .short('h')
                .long("human")
                .help("Prints numbers in human-readable way (124 kiB, 76 miB)")
        )
        .arg(
            Arg::new("ignore-general")
            .action(ArgAction::SetTrue)
                .long("ignore-general")
                .help("Provides only general info e.g name, size, when accessed...")
        )
        .arg(
            Arg::new("only-general")
            .action(ArgAction::SetTrue)
                .long("only-general")
                .help("Provide only special info e.g basic extension info, special metadata of file... (when with ignore-general provides only info of extension)")
        )
        .after_help("This app was written to analyze files, and give as much info about it as possible")
        .get_matches();
    
    let file_path : PathBuf = m.get_one::<PathBuf>("FILE").unwrap().clone();
    // Getting path to extensions.toml (forced to use env::current_dir())
    let mut extensions_path = env::current_dir().unwrap().clone();
    extensions_path.push("Extensions.toml");

    let args = Arguments
    { 
        file_path,
        extensions_path,
        is_debug : m.get_flag("debug"), 
        is_human : m.get_flag("human"), 
        only_general : m.get_flag("only-general"), 
        ignore_general: m.get_flag("ignore-general"), 
        extension_info: m.get_flag("extension-info") 
    };
    if args.is_debug { println!("Path to file: {:?}",&args.file_path); }

    // GUI interface (for now)
    let app = app::App::default();
    let mut wind = Window::new(100, 100, 400, 300, "FAT-RS v0.1.1");
    Frame::new(0, 0, 400, 200, "Program to analyze files");
    let mut but = Button::new(160, 210, 80, 40, "Load");
    wind.end();
    wind.show();


    // On pressing button we get info about file (selected from above)
    but.set_callback(move |_| get_info(&args));
    
    app.run().unwrap();
}