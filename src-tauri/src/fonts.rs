use font_kit::source::SystemSource;

pub fn system_font_families() -> Result<Vec<String>, String> {
    let mut families = SystemSource::new()
        .all_families()
        .map_err(|error| error.to_string())?;
    families.retain(|family| !family.trim().is_empty());
    families.sort();
    families.dedup();
    Ok(families)
}

#[tauri::command]
pub async fn list_system_fonts() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(system_font_families)
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::process::Command;

    #[test]
    fn catalog_matches_installed_fontconfig_families() {
        let output = Command::new("fc-list")
            .args(["--format", "%{family[0]}\n"])
            .output()
            .expect("fontconfig tools should be available");
        assert!(output.status.success());
        let expected: BTreeSet<String> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(str::trim)
            .filter(|family| !family.is_empty())
            .map(str::to_owned)
            .collect();
        let actual: BTreeSet<String> = system_font_families().unwrap().into_iter().collect();
        assert_eq!(actual, expected);
    }
}
