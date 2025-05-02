use indicatif::{ProgressBar, ProgressStyle};
use std::fmt::Display;

/// Create a progress bar with the specified message
pub fn create_progress_bar(message: &str) -> ProgressBar {
    let progress_bar = ProgressBar::new(0);
    
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("=> ")
    );
    
    progress_bar.set_message(message.to_string());
    
    progress_bar
}

/// Update the progress bar with a new message and position
pub fn update_progress(
    progress_bar: &ProgressBar,
    message: &str,
    position: u64,
    length: u64
) {
    progress_bar.set_message(message.to_string());
    progress_bar.set_position(position);
    progress_bar.set_length(length);
}

/// Update only the message of the progress bar without printing to console
pub fn update_message(progress_bar: &ProgressBar, message: impl Display) {
    progress_bar.set_message(message.to_string());
}

/// Print a message while temporarily suspending the progress bar
/// Only use this for important summary messages, not for individual records
pub fn print_with_progress(progress_bar: &ProgressBar, message: &str) {
    progress_bar.suspend(|| {
        println!("{}", message);
    });
}

/// Log an error message to the log file without printing to console
pub fn log_error(message: &str) {
    log::error!("{}", message);
}