//! Backward compatibility alias binary for Plinth BIM (`cargo run --bin plinth`)

fn main() -> eframe::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        plinth_app::run_app()
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(())
    }
}
