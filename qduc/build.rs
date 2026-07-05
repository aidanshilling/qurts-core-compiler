fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=cpp/lib/Qduc/Qduc.cpp");
    println!("cargo:rerun-if-changed=cpp/include/Qduc/QducDialect.h");

    // The C++ dialect library, pre-built by CMake
    println!("cargo:rustc-link-search=native={manifest}/cpp/build");
    println!("cargo:rustc-link-lib=static=QducDialect");

    // QducDialect.a is compiled C++ — needs the stdlib at final link
    println!("cargo:rustc-link-lib=c++");
}
