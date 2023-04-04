use nix::unistd;
use regex::Regex;
use std::env;
use std::error::Error;
use std::fs::{read_to_string, File};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::{Command, Stdio};
use tar::Archive;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Invalid arguments");
    }

    match args[1].as_str() {
        "run" => {
            let image = &args[2];
            let tar = format!("./assets/{}.tar.gz", image);

            if !Path::new(&tar).exists() {
                panic!("Tar file not found");
            }

            let cmd = if args.len() > 3 {
                args[3].clone()
            } else {
                read_to_string(format!("./assets/{}-cmd", image))?
            };

            let dir_path = create_temp_dir(&tar)?;

            un_tar(&tar, Path::new(dir_path.as_str()))?;
            chroot(Path::new(dir_path.as_str()), &cmd)?;
        }
        "pull" => {
            let image = &args[2];
            pull_image(image)?;
        }
        _ => panic!("Invalid command"),
    }

    Ok(())
}

fn pull_image(image: &str) -> Result<(), Box<dyn Error>> {
    println!("Pulling image ....{}", image);
    let mut cmd = Command::new("src/pull")
        .arg(image)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    cmd.wait()?;

    Ok(())
}

fn chroot(new_root: &Path, call: &str) -> Result<(), Box<dyn Error>> {
    // let old_root_handle = File::open("/")?;

    println!("Running {} in {:?}", call, new_root);
    unistd::fchdir(File::open(new_root)?.as_raw_fd())?;
    unistd::chroot(".")?;

    let mut cmd = Command::new("pwd")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    cmd.wait()?;

    let mut cmd = Command::new(call)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    cmd.wait()?;

    Ok(())
}

fn create_temp_dir(name: &str) -> Result<String, Box<dyn Error>> {
    let non_alphanumeric_regex = Regex::new(r"[^a-zA-Z0-9 ]+")?;
    Ok(non_alphanumeric_regex.replace_all(name, "_").to_string())
}

fn un_tar(source: &str, dst: &Path) -> Result<(), Box<dyn Error>> {
    println!("Unpacking {} to {:?} ...", source, dst);
    let tar_gz = File::open(source)?;
    match Archive::new(tar_gz).unpack(dst) {
        Ok(_) => println!("Unpacked"),
        Err(_e) => println!("The file is already unpacked."),
    }
    Ok(())
}
