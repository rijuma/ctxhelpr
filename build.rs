fn main() {
    let output = std::process::Command::new("date")
        .args(["+%Y-%m-%d"])
        .output();
    let date = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    };
    println!("cargo:rustc-env=CTXHELPR_BUILD_DATE={date}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=CTXHELPR_BUILD_TARGET={target}");
}
