
fn main() {
    linker_be_nice();
    // println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");

    // Generate blocklist.
    //
    // use std::collections::BTreeSet;
    // use std::env;
    // use std::fs::File;
    // use std::io::{BufWriter, Write};
    // use std::path::Path;
    //
    // println!("cargo:rerun-if-changed=build.rs");
    //
    // let out_file = env::var("BLOCKLIST").unwrap();
    // let dest_path = Path::new(&out_file);
    // let mut writer = BufWriter::new(File::create(dest_path).unwrap());
    //
    // // 1. Fetch blocklist at build time
    // let url = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts";
    // let body = reqwest::blocking::get(url)
    //     .expect("Failed to fetch blocklist")
    //     .text()
    //     .expect("Failed to read blocklist body");
    //
    // // 2. Parse, filter, deduplicate, and sort entries
    // let mut domains = BTreeSet::new();
    // for line in body.lines() {
    //     let line = line.trim();
    //     if line.starts_with('#') || line.is_empty() {
    //         continue;
    //     }
    //     let parts: Vec<&str> = line.split_whitespace().collect();
    //     if parts.len() >= 2 {
    //         let domain = parts[1].to_lowercase();
    //         if domain != "localhost" && domain != "broadcasthost" {
    //             domains.insert(domain);
    //         }
    //     }
    // }
    //
    // // 3. Emit a pre-sorted static array into blocklist
    // writeln!(writer, "pub static BLOCKED: [&str; {}] = [", domains.len()).unwrap();
    // for domain in &domains {
    //     writeln!(writer, "    \"{}\",", domain).unwrap();
    // }
    // writeln!(writer, "];").unwrap();
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                what if what.starts_with("_defmt_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                what if what.starts_with("esp_rtos_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                "free"
                | "malloc"
                | "calloc"
                | "get_free_internal_heap_size"
                | "malloc_internal"
                | "realloc_internal"
                | "calloc_internal"
                | "free_internal" => {
                    eprintln!();
                    eprintln!(
                        "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
                    );
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}
