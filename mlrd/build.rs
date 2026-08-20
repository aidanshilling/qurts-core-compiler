fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=cpp/lib/Mlrd/Mlrd.cpp");
    println!("cargo:rerun-if-changed=cpp/include/Mlrd/MlrdDialect.h");

    // The C++ dialect library, pre-built by CMake
    println!("cargo:rustc-link-search=native={manifest}/cpp/build");
    println!("cargo:rustc-link-lib=static=MlrdDialect");

    // mlrd.td reuses qauc::RefType/QubitType, so MlrdDialect depends on
    // QaucDialect, which itself depends on QducDialect. mlrd's CMake build
    // compiles both as nested subdirectories: cpp/build/qauc-build and
    // cpp/build/qauc-build/qduc-build.
    println!("cargo:rustc-link-search=native={manifest}/cpp/build/qauc-build");
    println!("cargo:rustc-link-lib=static=QaucDialect");

    println!("cargo:rustc-link-search=native={manifest}/cpp/build/qauc-build/qduc-build");
    println!("cargo:rustc-link-lib=static=QducDialect");

    // MlrdDialect.a is compiled C++ — needs the stdlib at final link
    println!("cargo:rustc-link-lib=c++");
}
