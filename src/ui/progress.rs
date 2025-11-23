use indicatif::{ProgressBar, ProgressStyle};

/// Create a spinner for indeterminate progress
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid spinner template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}



/// Reusable spinner wrapper with message updating capability
pub struct Spinner {
    pb: ProgressBar,
}

impl Spinner {
    /// Create a new spinner with an initial message
    pub fn new(message: &str) -> Self {
        Self {
            pb: spinner(message),
        }
    }
    
    /// Update the spinner message
    #[allow(dead_code)]
    pub fn update(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }
    
    /// Finish the spinner and clear it
    #[allow(dead_code)]
    pub fn finish(self) {
        self.pb.finish_and_clear();
    }
    
    /// Finish the spinner with a message
    #[allow(dead_code)]
    pub fn finish_with_message(self, message: &str) {
        self.pb.finish_with_message(message.to_string());
    }
    
    /// Get a reference to the underlying ProgressBar (for compatibility)
    pub fn inner(&self) -> &ProgressBar {
        &self.pb
    }
}
