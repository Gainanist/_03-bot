use chrono::Utc;

pub fn format_time() -> String {
    Utc::now().format("%H:%M:%S").to_string()
}
