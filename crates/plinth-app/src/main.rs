//! # Arch3D Desktop Application Binary

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
