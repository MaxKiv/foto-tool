#[cfg(feature = "libChafa")]
use chafa::ChafaCanvas;

use chrono::{DateTime, Local, NaiveDate};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::DirEntry;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;
use std::{env, fs, io};

const VALID_FILE_EXTENTIONS: [&str; 3] = ["jpeg", "jpg", "mp4"];

#[derive(Debug)]
struct ImageFileMap {
    map: BTreeMap<NaiveDate, Vec<PathBuf>>,
}
impl ImageFileMap {
    fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }
}

enum UserOptions {
    Exit,
    NextImage,
    PreviousImage,
    CityName(String),
}

fn main() -> Result<(), Box<dyn Error>> {
    let current_dir = env::current_dir()?;
    println!("The current working directory is: {:?}", current_dir);
    ask_user_confirmation()?;

    let grouped_images = group_images_in_dir(current_dir)?;

    let mut image_idx = 0;
    for (datetime, images) in grouped_images.map.iter() {
        loop {
            println!("{:?}", images);
            println!("image idx: {image_idx}");
            // Loop through images based on user input
            let current_image = &images[image_idx];

            display_with_chafa(current_image)?;

            use UserOptions::*;
            match ask_user_directory_name()? {
                Exit => return Err(Box::from("Operation aborted by the user.")),
                NextImage => image_idx = (image_idx + 1) % images.len(),
                PreviousImage => {
                    image_idx = if image_idx == 0 {
                        images.len() - 1
                    } else {
                        image_idx - 1
                    }
                }
                CityName(city_name) => {
                    create_dir_and_copy_images(datetime, city_name, images)?;
                    break;
                }
            }
        }
    }

    Ok(())
}

fn create_dir_and_copy_images(
    datetime: &NaiveDate,
    city_name: String,
    images: &[PathBuf],
) -> Result<(), Box<dyn Error>> {
    // Format the date as "DD-MM-YYYY"
    let formatted_date = datetime.format("%d-%m-%Y").to_string();

    // Format the full directory name as "DD-MM-YYYY_CITY"
    let full_dir_name = format!("{}_{}", formatted_date, city_name);
    // println!("The full directory string is {}", full_dir_name);

    // The directory we want to create
    let image_dir = Path::new(&full_dir_name);

    fs::create_dir_all(image_dir)?;

    for image in images {
        if let Some(file_name) = image.file_name() {
            let dest_path = image_dir.join(file_name);
            // move the file
            fs::rename(image, &dest_path)?;
            println!("Moved {:?} to {:?}", image, dest_path);
        } else {
            eprintln!("Failed to get file name for {:?}", image);
        }
    }

    Ok(())
}

fn group_images_in_dir(current_dir: PathBuf) -> Result<ImageFileMap, Box<dyn Error>> {
    // Collection of Files, grouped by their day modified
    let mut grouped_images: ImageFileMap = ImageFileMap::new();

    // Iterate over all files in the directory
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if is_image_file(&path) {
            let modified_date = get_file_modified_date(entry, &path)?;

            let modified_date: DateTime<Local> = modified_date.into();
            let modified_date = modified_date.date_naive();

            // Insert the file path into the map based on the modified date
            grouped_images
                .map
                .entry(modified_date)
                .or_default()
                .push(path);
        }
    }

    Ok(grouped_images)
}

fn is_image_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| VALID_FILE_EXTENTIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
}

fn get_file_modified_date(entry: DirEntry, path: &PathBuf) -> Result<SystemTime, Box<dyn Error>> {
    // Get the modified time of the file or return an error if it fails
    let metadata = entry
        .metadata()
        .map_err(|_| format!("Error: unable to get metadata for {:?}", path))?;

    let modified_date = metadata
        .modified()
        .map_err(|_| format!("Error: unable to get modified date for {:?}", metadata))?;

    println!("{:?}", modified_date);

    Ok(modified_date)
}

/// Ask the user if they want to continue
fn ask_user_confirmation() -> Result<(), Box<dyn Error>> {
    loop {
        print!("Do you want to continue? (y/n): ");
        io::stdout().flush()?; // Flush to ensure the prompt is displayed before reading input

        // Read the user's input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "y" => {
                println!("Continuing");
                return Ok(());
            }
            "n" => {
                println!("Exiting");
                return Err(Box::from("Operation aborted by the user."));
            }
            _ => {
                println!("Invalid input, please enter 'y' or 'n'.");
            }
        }
    }
}

/// Ask the user if they want to continue
fn ask_user_directory_name() -> Result<UserOptions, Box<dyn Error>> {
    use UserOptions::*;

    loop {
        println!("What city was this?");
        println!("Enter city name, or enter 'n' for next image, or 'p' for previous. 'q' to quit");
        io::stdout().flush()?; // Flush to ensure the prompt is displayed before reading input

        // Read the user's input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "n" => {
                println!("Showing next image");
                return Ok(NextImage);
            }
            "p" => {
                println!("Showing previous image");
                return Ok(PreviousImage);
            }
            "q" => {
                println!("Exiting");
                return Ok(Exit);
            }
            user_input_city_name => {
                if !user_input_city_name.is_empty() {
                    println!("City entered: {}", user_input_city_name);
                    return Ok(CityName(user_input_city_name.to_string()));
                } else {
                    println!("Enter a valid city name");
                }
            }
        }
    }
}
fn display_with_chafa(image_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    #[cfg(not(feature = "libChafa"))]
    {
        Command::new("chafa")
            .arg(image_path)
            .status()
            .expect("Failed to display file with chafa");

        Ok(())
    }
    #[cfg(feature = "libChafa")]
    {
        let img = image::open(image_path).expect("failed to open image");

        let canvas = ChafaCanvas::from_term(img.width(), img.height());
        let pixels = img.to_rgba8();
        let ansi = canvas.draw(&pixels, img.width(), img.height());

        // Print the rendered output to the terminal
        println!("{}", ansi);

        Ok(())
    }
}
