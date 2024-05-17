use std::error::Error;
use std::fs::File;
use std::io;
use std::io::stdout;
use std::io::Cursor;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use image::ImageFormat::Bmp;
use image::ImageFormat::Png;

use crate::ask;

pub fn main(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let mut args = args.into_iter();

    let output = match args.next() {
        Some(x) => x,
        None => ask("Output path (Default: `-` to print)")?,
    };

    // TODO other platforms
    #[cfg(target_os = "windows")]
    let buf = snip()?;

    #[cfg(not(target_os = "windows"))]
    return Err(format!(
        "Cannot make a screen snip on your platform ({})",
        std::env::consts::OS
    )
    .into());

    let image = image::load_from_memory_with_format(&buf, Bmp)?;

    let text = {
        let mut buf = Cursor::new(Vec::<u8>::new());
        image.write_to(&mut buf, Png)?;

        let child = Command::new("tesseract")
            .arg("-")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .as_ref()
            .unwrap()
            .write_all(&buf.into_inner()[..])?;
        child
            .wait_with_output()?
            .stdout
    };

    let mut output: Box<dyn Write> = match &*output {
        "" | "-" => Box::new(stdout()),
        path => Box::new(
            match File::create_new(&path) {
                Ok(file) => file,
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    let answer = ask("File already exists. Overwrite? (`y` / anything else)")?
                        .to_lowercase();
                    if answer != "y" && answer != "yes" {
                        return Ok(());
                    }
                    File::create(&path)?
                },
                Err(err) => return Err(err.into()),
            },
        ),
    };
    output.write_all(&text)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn snip() -> Result<Vec<u8>, Box<dyn Error>> {
    use std::thread::sleep;
    use std::time::Duration;

    use clipboard_win::formats;
    use clipboard_win::get_clipboard;
    use sysinfo::ProcessRefreshKind;
    use sysinfo::RefreshKind;
    use sysinfo::System;

    Command::new("explorer")
        .arg("ms-screenclip:")
        .spawn()?;

    sleep(Duration::from_millis(1000));

    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    while system
        .processes()
        .into_iter()
        .flat_map(|(_, p)| p.exe())
        .flat_map(|exe| exe.file_name())
        .filter(|exe| exe.to_string_lossy() == "ScreenClippingHost.exe")
        .next()
        .is_some()
    {
        sleep(Duration::from_millis(100));
        system.refresh_processes();
    }

    get_clipboard(formats::Bitmap).map_err(|e| format!("Failed to read clipboard: {e}").into())
}
