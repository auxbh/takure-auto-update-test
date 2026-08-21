use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        .git_describe(false, true, None)
        .git_sha(false)
        .build_date()
        .emit()?;

    // Makes it possible to write function pointers to the .rtext section
    if std::env::var("TARGET").unwrap_or_default().ends_with("msvc") {
        println!("cargo::rustc-link-arg=/SECTION:.rtext,RW");
    } else {
        println!("cargo::warning=Not targeting MSVC: the .rtext section will not be writable, which will cause a crash if a self-update is triggered.");
    }

    winresource::WindowsResource::new().compile()?;

    Ok(())
}