//! Build information for Settings › About (git commit, build date), and the app
//! icon and version info in the Windows executable (macOS and Linux get the icon
//! from the .app bundle / .desktop file).

fn main() {
    println!("cargo:rerun-if-changed=../../assets/app-icon/kadeestudio.ico");
    build_info();
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/app-icon/kadeestudio.ico");
        res.set("ProductName", "KaDee Platform");
        res.set("FileDescription", "KadeeStudio");
        res.set("CompanyName", "Kris Dugardyn");
        res.set("LegalCopyright", "MIT OR Apache-2.0");
        if let Err(e) = res.compile() {
            // A missing resource compiler should not break the build; the exe just has no icon.
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}

/// `KADEE_COMMIT` (short hash, `+` when there are uncommitted changes) and `KADEE_BUILD_DATE`.
fn build_info() {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let commit = git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    println!("cargo:rustc-env=KADEE_COMMIT={commit}{}", if dirty { "+" } else { "" });
    // SOURCE_DATE_EPOCH keeps reproducible builds reproducible.
    let secs = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| git(&["log", "-1", "--format=%ct"]).and_then(|s| s.parse().ok()))
        .unwrap_or(0);
    println!("cargo:rustc-env=KADEE_BUILD_DATE={}", civil_date(secs));
    if let Some(head) = git(&["rev-parse", "--git-dir"]) {
        println!("cargo:rerun-if-changed={head}/HEAD");
        println!("cargo:rerun-if-changed={head}/index");
    }
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
}

/// `YYYY-MM-DD` for a Unix time (UTC), without a date crate.
fn civil_date(secs: i64) -> String {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
