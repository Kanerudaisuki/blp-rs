use std::error::Error;
use std::path::Path;

/// Проверка, что входной файл существует
pub fn ensure_input_exists(input: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !input.exists() {
        return Err(format!("input not found: {}", input.display()).into());
    }
    Ok(())
}
