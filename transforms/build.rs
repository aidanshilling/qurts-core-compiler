use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let build = manifest.join("cpp/build");

    println!("cargo:rerun-if-changed=cpp/lib");
    println!("cargo:rerun-if-changed=cpp/include");
    println!("cargo:rerun-if-changed=td/passes.td");

    println!("cargo:rustc-link-search=native={}", build.display());
    println!("cargo:rustc-link-lib=static=QurtsTransforms");

    for (dir, lib) in [
        ("mlrd-build", "MlrdDialect"),
        ("mlrd-build/qauc-build", "QaucDialect"),
        ("mlrd-build/qauc-build/qduc-build", "QducDialect"),
    ] {
        println!("cargo:rustc-link-search=native={}", build.join(dir).display());
        println!("cargo:rustc-link-lib=static={lib}");
    }

    let mut archives = Vec::new();
    collect_archives(&build.join("mqt-core-build"), &mut archives);
    let mut dirs = BTreeSet::new();
    let mut libs = BTreeSet::new();
    for archive in archives {
        let name = archive.file_stem().unwrap().to_str().unwrap().trim_start_matches("lib").to_string();
        if name == "foo" {
            continue;
        }
        dirs.insert(archive.parent().unwrap().to_path_buf());
        libs.insert(name);
    }
    for dir in dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    for lib in libs {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    println!("cargo:rustc-link-lib=c++");
}

fn collect_archives(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_archives(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "a") {
            out.push(path);
        }
    }
}
