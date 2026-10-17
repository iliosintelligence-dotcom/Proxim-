//! Pack golden vectors (`docs/pack-format.md`, engine spec 9).
//!
//! `cargo run -p proxim-pack --features testkit --example gen_pack_vectors` compares with the file on
//! disk and exits non-zero on a difference; `PROXIM_BLESS=1` rewrites it. Afterwards refresh
//! `MANIFEST.json` with `PROXIM_BLESS=1 cargo run -p proxim-decide --example gen_golden`.
#![allow(clippy::print_stdout, clippy::unwrap_used)]

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let text = common::vectors::render();
    let path = common::vectors::golden_path();
    if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
        println!("same    {}", path.display());
        return;
    }
    if std::env::var("PROXIM_BLESS").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
        println!("written {} ({} bytes)", path.display(), text.len());
    } else {
        println!(
            "DIFFERS {} (run with PROXIM_BLESS=1 to write)",
            path.display()
        );
        std::process::exit(1);
    }
}
