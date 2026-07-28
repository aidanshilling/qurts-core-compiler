fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=cpp/lib/Qauc/Qauc.cpp");
    println!("cargo:rerun-if-changed=cpp/include/Qauc/QaucDialect.h");

    // The C++ dialect library, pre-built by CMake
    println!("cargo:rustc-link-search=native={manifest}/cpp/build");
    println!("cargo:rustc-link-lib=static=QaucDialect");

    // qauc.td reuses qduc::LifetimeType, so QaucDialect depends on QducDialect.
    // qauc's CMake build compiles qduc as a subdirectory into cpp/build/qduc-build.
    println!("cargo:rustc-link-search=native={manifest}/cpp/build/qduc-build");
    println!("cargo:rustc-link-lib=static=QducDialect");

    // QaucDialect.a is compiled C++ — needs the stdlib at final link
    println!("cargo:rustc-link-lib=c++");
}
