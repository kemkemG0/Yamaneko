use std::{
    env,
    error::Error,
    fs::{read_to_string, File},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Invalid arguments");
    }

    match args[1].as_str() {
        "run" => run(args),
        "pull" => pull(args),
        _ => panic!("Invalid command"),
    }
}

fn run(args: Vec<String>) {
    let images_base_path = "./assets/images";
    let image = &args[2];
    let tar = format!("{}/{}/{}.tar.gz", images_base_path, image, image);

    if !Path::new(&tar).exists() {
        panic!("Tar file not found");
    }

    let cmd = if args.len() > 3 {
        args[3].clone()
    } else {
        read_to_string(format!("{}/{}/{}-cmd", images_base_path, image, image))
            .expect("Commane not found")
    };

    let container_path = create_container_path(image).expect("Error creating container path");
    un_tar(&tar, Path::new(container_path.as_str()));
    chroot(Path::new(container_path.as_str()), &cmd).expect("Error running command");
}

fn pull(args: Vec<String>) {
    let image = &args[2];
    match pull_image(image) {
        Ok(_) => println!("Image pulled"),
        Err(e) => eprintln!("Error pulling image: {}", e),
    }
}

fn execute_command(call: &str, arg: Option<&str>) -> std::io::Result<ExitStatus> {
    let mut cmd = Command::new(call);
    if let Some(a) = arg {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("Error: {:?}\n Command: {} execution failed", e, call))
        .wait()
}

fn pull_image(image: &str) -> std::io::Result<ExitStatus> {
    println!("Pulling image ....{}", image);
    execute_command("./src/pull", Some(image))
}

fn chroot(new_root: &Path, call: &str) -> std::io::Result<ExitStatus> {
    println!("Running {} in {:?}", call, new_root);

    match nix::unistd::chroot(new_root) {
        Ok(_) => println!("chroot success"),
        Err(e) => panic!("chroot failed: {}", e),
    }
    execute_command(call, None)
}

fn create_container_path(image_name: &str) -> Result<String, Box<dyn Error>> {
    Ok(format!(
        "{}/{}",
        "./assets/containers",
        regex::Regex::new(r"[^a-zA-Z0-9 ]+")?.replace_all(image_name, "_")
    ))
}

fn un_tar(source: &str, dst: &Path) {
    println!("Unpacking {} to {:?} ...{}", source, dst, dst.exists());
    if dst.exists() {
        println!("Already unpacked");
        return;
    }
    match tar::Archive::new(match File::open(source) {
        Ok(f) => f,
        Err(e) => panic!("Error opening file: {}", e),
    })
    .unpack(dst)
    {
        Ok(_) => println!("Unpacked"),
        Err(_) => println!("Already unpacked"),
    };
}
