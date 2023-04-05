use std::{
    env,
    error::Error,
    fs::{read_to_string, File},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

const DEFAULT_SHELL: &str = "/bin/bash";

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
    let tar = format!("{images_base_path}/{image}/{image}.tar.gz");

    if !Path::new(&tar).exists() {
        panic!("Tar file not found");
    }

    let cmd = if args.len() > 3 {
        args.clone()
            .into_iter()
            .skip(3)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        read_to_string(format!("{images_base_path}/{image}/{image}-cmd"))
            .expect("Commane not found")
    };

    let container_path = create_container_path(image).expect("Error creating container path");
    un_tar(&tar, Path::new(container_path.as_str()));
    unshare_chroot_mount(Path::new(container_path.as_str()), &cmd).expect("Error running command");
}

fn pull(args: Vec<String>) {
    let image = &args[2];
    match pull_image(image) {
        Ok(_) => println!("Image pulled"),
        Err(e) => eprintln!("Error pulling image: {}", e),
    }
}

fn execute_command(command: &str, args: Vec<&str>) -> std::io::Result<ExitStatus> {
    let mut cmd = Command::new(command);

    println!("command: {}\nargs:{:?}", command, args);

    cmd.stdin(Stdio::inherit())
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("Error: {:?}\n Command: {command} execution failed", e))
        .wait()
}

fn pull_image(image: &str) -> std::io::Result<ExitStatus> {
    println!("Pulling image ....{image}");
    execute_command("./src/pull", vec![image])
}

fn exec_shell_and_mount_proc_and_exec_command(command: &str) -> std::io::Result<ExitStatus> {
    let arg = format!("mount -t proc proc /proc && {DEFAULT_SHELL} -c '{command}'");
    execute_command(DEFAULT_SHELL, vec!["-c", &arg])
}

fn chroot(new_root: &Path, command: &str) -> std::io::Result<ExitStatus> {
    println!("Running {} in {:?}", command, new_root);
    nix::unistd::fchdir(std::os::fd::AsRawFd::as_raw_fd(&File::open(new_root)?))?;

    match nix::unistd::chroot(".") {
        Ok(_) => println!("chroot success"),
        Err(e) => panic!("chroot failed: {e}"),
    }
    exec_shell_and_mount_proc_and_exec_command(command)
}

fn unshare_chroot_mount(new_root: &Path, command: &str) -> std::io::Result<ExitStatus> {
    // // Unshare the mount, PID, and network namespaces
    use nix::sched::{unshare, CloneFlags};
    unshare(CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNET)
        .expect("Failed to unshare namespaces");
    chroot(Path::new(new_root), command)
}

fn create_container_path(image_name: &str) -> Result<String, Box<dyn Error>> {
    Ok(format!(
        "{}/{}",
        "./assets/containers",
        regex::Regex::new(r"[^a-zA-Z0-9 ]+")?.replace_all(image_name, "_")
    ))
}

fn un_tar(source: &str, dst: &Path) {
    println!("Unpacking {source} to {:?} ...{}", dst, dst.exists());
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
