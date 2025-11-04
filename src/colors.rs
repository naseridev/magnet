use owo_colors::OwoColorize;
use std::sync::OnceLock;

static COLOR_ENABLED: OnceLock<bool> = OnceLock::new();

pub fn init_colors() {
    COLOR_ENABLED.get_or_init(|| {
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        supports_color::on(supports_color::Stream::Stdout)
            .map(|level| level.has_basic)
            .unwrap_or(false)
    });
}

fn is_enabled() -> bool {
    *COLOR_ENABLED.get_or_init(|| {
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        supports_color::on(supports_color::Stream::Stdout)
            .map(|level| level.has_basic)
            .unwrap_or(false)
    })
}

pub fn success(text: &str) -> String {
    if is_enabled() {
        text.bright_blue().to_string()
    } else {
        text.to_string()
    }
}

pub fn error(text: &str) -> String {
    if is_enabled() {
        text.white().to_string()
    } else {
        text.to_string()
    }
}

pub fn warning(text: &str) -> String {
    if is_enabled() {
        text.white().to_string()
    } else {
        text.to_string()
    }
}

pub fn info(text: &str) -> String {
    if is_enabled() {
        text.bright_blue().to_string()
    } else {
        text.to_string()
    }
}

pub fn bold(text: &str) -> String {
    if is_enabled() {
        text.bold().to_string()
    } else {
        text.to_string()
    }
}

pub fn dim(text: &str) -> String {
    if is_enabled() {
        text.white().to_string()
    } else {
        text.to_string()
    }
}

pub fn header(text: &str) -> String {
    if is_enabled() {
        text.bright_blue().bold().to_string()
    } else {
        text.to_string()
    }
}

pub fn package_name(text: &str) -> String {
    if is_enabled() {
        text.bright_blue().bold().to_string()
    } else {
        text.to_string()
    }
}

pub fn version(text: &str) -> String {
    if is_enabled() {
        text.white().to_string()
    } else {
        text.to_string()
    }
}

pub fn path(text: &str) -> String {
    if is_enabled() {
        text.bright_blue().to_string()
    } else {
        text.to_string()
    }
}

pub fn status_prefix(text: &str, status_type: StatusType) -> String {
    if !is_enabled() {
        return format!("{:>12} ", text);
    }

    let colored = match status_type {
        StatusType::Success => text.bright_blue().bold().to_string(),
        StatusType::Error => text.white().bold().to_string(),
        StatusType::Warning => text.white().bold().to_string(),
        StatusType::Info => text.bright_blue().bold().to_string(),
        StatusType::Progress => text.bright_blue().bold().to_string(),
    };

    format!("{:>12} ", colored)
}

pub enum StatusType {
    Success,
    Error,
    Warning,
    Info,
    Progress,
}

pub fn print_success(prefix: &str, message: &str) {
    println!("{}{}", status_prefix(prefix, StatusType::Success), message);
}

pub fn print_error(prefix: &str, message: &str) {
    eprintln!("{}{}", status_prefix(prefix, StatusType::Error), message);
}

pub fn print_warning(prefix: &str, message: &str) {
    println!("{}{}", status_prefix(prefix, StatusType::Warning), message);
}

pub fn print_info(prefix: &str, message: &str) {
    println!("{}{}", status_prefix(prefix, StatusType::Info), message);
}

pub fn print_progress(prefix: &str, message: &str) {
    println!("{}{}", status_prefix(prefix, StatusType::Progress), message);
}
