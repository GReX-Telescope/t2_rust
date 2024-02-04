use clap::Parser;

/// T2 Processing for GReX (clustering and filtering of Heimdall candidates)
#[derive(Parser, Debug)]
pub struct Args {
    /// Minimum DM to filter
    #[arg(long, default_value_t = 20.0)]
    pub min_dm: f32,
    /// Maximum DM to filter
    #[arg(long, default_value_t = 3000.0)]
    pub max_dm: f32,
    /// Minimum SNR to filter
    #[arg(long, default_value_t = 20.0)]
    pub min_snr: f32,
    /// Database URL
    #[arg(long)]
    pub url: String,
}
