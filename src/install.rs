use std::fs;
use std::path::PathBuf;

fn install_local_package(source_path: PathBuf, target_dir: PathBuf) {
    let file_name = source_path.file_name().unwrap();
    let target_path = target_dir.join(file_name);

    fs::copy(&source_path, &target_path).expect("Failed to copy file");

    println!("Installed local package: {:?}", target_path);
}

fn install_remote_package(_package_name: String, _target_dir: PathBuf) {
    unimplemented!();
}

pub fn install_package(package_or_path: String) {
    let path = PathBuf::from(&package_or_path);
    let target_dir = PathBuf::from("./.sag_packages/");

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).expect("Failed to create package directory");
    }

    if path.exists() {
        install_local_package(path, target_dir);
    } else {
        install_remote_package(package_or_path, target_dir);
    }
}
