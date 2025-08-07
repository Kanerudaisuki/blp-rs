use blp_rs::ui::run::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().map_err(|e| {
        eprintln!("💥 Failed to run: {}", e);
        e
    })
}
