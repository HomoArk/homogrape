use anyhow::Result;
use std::{env, path::PathBuf};

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<()> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("dist") => {
            if let Some(dest) = env::args().nth(2) {
                dist(dest)?;
            } else {
                print_help();
            }
        }
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

dist            invoke `ohrs build` and copy generated files to the given directory
"
    )
}

fn dist(dest: String) -> Result<()> {
    let dest = dunce::canonicalize(PathBuf::from(dest))?;

    let root = project_root();

    let _ = std::process::Command::new("ohrs")
        .current_dir(&root)
        .arg("build")
        .arg("--arch=aarch")
        // .arg("--arch=x86_64")
        .status()
        .expect("failed to build the project");

    // copy dist/arm64-v8a/*.so to ../../../libs/arm64-v8a/
    let _ = std::fs::create_dir_all(&dest);
    let src = dunce::canonicalize(&root.join("dist/arm64-v8a"))?;
    let files = std::fs::read_dir(&src)?;
    for file in files {
        let file = file?;
        let path = file.path();
        if !path.is_file() {
            continue;
        }
        let dest = dest.join(path.file_name().unwrap());
        match std::fs::copy(&path, &dest) {
            Ok(_) => {
                println!(
                    "Copied {:?} to {:?}",
                    dunce::canonicalize(&path)?,
                    dunce::canonicalize(&dest)?
                );
            }
            Err(e) => {
                println!("failed to copy {:?} to {:?}: {}", path, dest, e);
            }
        }
    }

    Ok(())
}

fn project_root() -> PathBuf {
    let dir =
        env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir).parent().unwrap().to_owned()
}
