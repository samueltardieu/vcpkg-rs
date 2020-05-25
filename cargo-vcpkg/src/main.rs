use anyhow::bail;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    process::{Command, Stdio},
};
use vcpkg::{find_vcpkg_root, Config};

// settings for a specific Rust target
#[derive(Debug, Deserialize)]
struct Target {
    #[serde(default = "Vec::new")]
    install: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Vcpkg {
    vcpkg_root: Option<String>,
    target: BTreeMap<String, Target>,
    branch: Option<String>,
    rev: Option<String>,
    git: Option<String>,
    tag: Option<String>,
    #[serde(default = "Vec::new")]
    install: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct Metadata {
    vcpkg: Vcpkg,
}

fn main() {
    run().unwrap_or_else(|e| {
        eprintln!("cargo-vcpkg: {}", e);
        std::process::exit(1);
    });
}
fn run() -> Result<(), anyhow::Error> {
    let mut args = std::env::args().skip_while(|val| !val.starts_with("--manifest-path"));

    let mut cmd = cargo_metadata::MetadataCommand::new();
    match args.next() {
        Some(p) if p == "--manifest-path" => {
            cmd.manifest_path(args.next().unwrap());
        }
        Some(p) => {
            cmd.manifest_path(p.trim_start_matches("--manifest-path="));
        }
        None => {}
    }
    let metadata = cmd.exec().unwrap();

    let mut git_url = None;
    //let vcpkg_ports = Vec::new();
    for p in &metadata.packages {
        if let Ok(v) = serde_json::from_value::<Metadata>(p.metadata.clone()) {
            let v = v.vcpkg;
            git_url = v.git.clone();
            // dbg!(&p);
            dbg!(&p.metadata);

            // TODO: check the target and use it's package set if required

            let x = match (&v.branch, &v.tag, &v.rev) {
                (Some(b), None, None) => b,
                (None, Some(t), None) => t,
                (None, None, Some(r)) => r,
                _ => {
                    bail!("must specify one of branch,rev,tag for git source");
                }
            };

            dbg!(v);
        }
    }

    // should we modify the existing?
    // let mut allow_updates = true;

    // find the vcpkg root
    let vcpkg_root = find_vcpkg_root(&Config::default()).unwrap_or_else(|_| {
        let target_directory = metadata.target_directory.clone();
        let mut vcpkg_root = target_directory.clone();
        vcpkg_root.push("vcpkg");
        vcpkg_root.to_path_buf();
        vcpkg_root
    });

    // if it does not exist, clone vcpkg from git
    let mut vcpkg_root_file = vcpkg_root.clone();
    vcpkg_root_file.push(".vcpkg-root");
    if !vcpkg_root_file.exists() {
        // TODO: create target dir if it does not exist - don't need to, git does it?
        // dbg!(vcpkg_root_file);
        //let git_url = env::var("VCPKGRS_VCPKG_GIT_URL");
        //let git_url = "https://github.com/microsoft/vcpkg";
        let git_url = git_url.unwrap();
        let output = Command::new("git")
            .arg("clone")
            .arg(git_url)
            .arg(&vcpkg_root)
            //.stdout(Stdio::inherit())
            .stdout(Stdio::inherit())
            .output()
            .expect("failed to execute process");
        eprintln!("git clone = {:?}", output.status);
        println!("{:?}", output);
    } else {
        let output = Command::new("git")
            .arg("fetch")
            .arg("--verbose")
            .arg("--all")
            .stdout(Stdio::inherit())
            .output()
            .expect("failed to execute process");
        if output.status.success() {
            println!("Fetch succeeded");
        } else {
            eprintln!("git fetch = {:?}", output.status);
            eprintln!("{:?}", output);
        }
    }
    // otherwise, check that the rev is where we want it to be
    // there needs to be some serious thought here because if we are on a branch
    // does this mean we should fetch?

    // gotta get this from the metadata in the target package
    let rev_tag_branch = "4c1db68";

    // check out the required rev
    let output = Command::new("git")
        .arg("checkout")
        .arg(rev_tag_branch)
        .stdout(Stdio::inherit())
        .current_dir(&vcpkg_root)
        .output()
        .expect("failed to execute process");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    println!("{}", String::from_utf8_lossy(&output.stderr));

    // try and run 'vcpkg update' and if it fails or gives the version warning
    // rebuild it
    let mut vcpkg_command = Command::new("./vcpkg");
    //vcpkg_command.current_dir(&vcpkg_root);

    let require_bootstrap = match vcpkg_command
        //.clone()
        .arg("update")
        .current_dir(&vcpkg_root)
        .stdout(Stdio::inherit())
        .output()
    {
        Ok(output) if output.status.success() => false,
        Ok(output) => {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("{}", String::from_utf8_lossy(&output.stderr));
            println!("{:?}", output.status);
            true
        }
        Err(_) => true,
    };

    // build vcpkg
    // if cfg!(windows) {

    // }

    if require_bootstrap {
        let output = Command::new("sh")
            .arg("-c")
            .arg("./bootstrap-vcpkg.sh")
            .current_dir(&vcpkg_root)
            .stdout(Stdio::inherit())
            .output()
            .expect("failed to run vcpkg bootstrap");
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    let mut vcpkg_command = Command::new("./vcpkg");
    //vcpkg_command.current_dir(&vcpkg_root);

    let output = vcpkg_command
        //.clone()
        .arg("install")
        .arg("openssl")
        .current_dir(&vcpkg_root)
        .stdout(Stdio::inherit())
        .output()
        .expect("failed to execute process");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    println!("{}", String::from_utf8_lossy(&output.stderr));

    // done
    println!("done");
    Ok(())
}

// Warning: Different source is available for vcpkg

/*

{
  "packages": [
    {
      "name": "serde",
      "version": "1.0.110",
      "id": "serde 1.0.110 (registry+https://github.com/rust-lang/crates.io-index)",
      "license": "MIT OR Apache-2.0",
      "license_file": null,
      "description": "A generic serialization/deserialization framework",
      "source": "registry+https://github.com/rust-lang/crates.io-index",
      "dependencies": [
        {
          "name": "serde_derive",
          "source": "registry+https://github.com/rust-lang/crates.io-index",
          "req": "= 1.0.110",
          "kind": null,
          "rename": null,
          "optional": true,
          "uses_default_features": true,
          "features": [],
          "target": null,
          "registry": null
        },
        {
          "name": "serde_derive",
          "source": "registry+https://github.com/rust-lang/crates.io-index",
          "req": "^1.0",
          "kind": "dev",
          "rename": null,
          "optional": false,
          "uses_default_features": true,
          "features": [],
          "target": null,
          "registry": null
        }
      ],
      "targets": [
        {
          "kind": [
            "lib"
          ],
          "crate_types": [
            "lib"
          ],
          "name": "serde",
          "src_path": "/Users/jim/.cargo/registry/src/github.com-1ecc6299db9ec823/serde-1.0.110/src/lib.rs",
          "edition": "2015",
          "doctest": true
        },
*/
