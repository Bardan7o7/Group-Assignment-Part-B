use std::io::{self, Write};
use safe_backup::{backup_file, restore_file, delete_file, validate_path};

fn prompt(s: &str) -> io::Result<String> {
    print!("{s}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

fn main() -> io::Result<()> {
    loop {
        let filename = prompt("Please enter your file name: ")?;
        if filename.eq_ignore_ascii_case("exit") || filename.eq_ignore_ascii_case("quit") {
            println!("Bye.");
            break;
        }

        if let Err(e) = validate_path(&filename) {
            eprintln!("[error] {e}");
            continue;
        }

        let command = prompt("Please enter your command (backup, restore, delete): ")?;
        match command.to_lowercase().as_str() {
            "backup" => match backup_file(&filename) {
                Ok(path) => println!("Your backup created: {}", path.file_name().unwrap().to_string_lossy()),
                Err(e) => eprintln!("[error] {e}"),
            },
            "restore" => match restore_file(&filename) {
                Ok(dest) => println!("Your file has been restored: {}", dest.file_name().unwrap().to_string_lossy()),
                Err(e) => eprintln!("[error] {e}"),
            },
            "delete" => match delete_file(&filename) {
                Ok(_) => println!("Deleted: {filename}"),
                Err(e) => eprintln!("[error] {e}"),
            },
            other => eprintln!("[error] unknown command: {other}"),
        }
        println!();
    }
    Ok(())
}
