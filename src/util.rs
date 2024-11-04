pub fn is_empty(s: &str) -> bool {
    s.replace('_', "").trim().is_empty()
}

pub fn sanitize(name: &str) -> String {
    const INVALID_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
    let mut filename = name
        .chars()
        .map(|c| if INVALID_CHARS.contains(&c) { '_' } else { c })
        .collect::<String>();

    #[cfg(target_os = "windows")]
    {
        const RESERVED_NAMES: &[&str] = &[
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        if RESERVED_NAMES.contains(&filename.as_str()) {
            filename.push('_');
        }
    }

    if filename.len() > 255 {
        filename.truncate(255);
    }

    filename
}

pub fn prompt(msg: &str) -> bool {
    use std::io::{self, Write};

    print!("{} [Y/n]: ", msg);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().to_lowercase() == "y" || input.trim().is_empty()
}
