mod app;
mod diff_view;
mod dir_view;
mod merge_view;

use clap::Parser;
use eframe::egui;

#[derive(Parser, Debug)]
#[command(name = "kdiff3", about = "File comparison and merge tool")]
struct Args {
    /// Base file (A)
    file1: Option<String>,
    /// File B
    file2: Option<String>,
    /// File C (optional, for 3-way merge)
    file3: Option<String>,
    /// Output file (enables merge mode)
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("KDiff3-rs")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "kdiff3-rs",
        options,
        Box::new(move |cc| {
            let app = app::KDiff3App::new(cc, args.file1, args.file2, args.file3, args.output);
            Box::new(app) as Box<dyn eframe::App>
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
