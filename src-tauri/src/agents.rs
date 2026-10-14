use serde::Serialize;
use std::fs;
use std::io;
use std::path::PathBuf;

const HOOK_START: &str = "# >>> typelet: codex-prompt-start";
const HOOK_END: &str = "# <<< typelet: codex-prompt-start";
const CODEX_HOOK: &str = r#"# >>> typelet: codex-prompt-start
[[hooks.UserPromptSubmit]]
matcher = "*"

[[hooks.UserPromptSubmit.hooks]]
type = "command"
command = "nohup typelet >/dev/null 2>&1 </dev/null &"
command_windows = "Start-Process typelet"
timeout = 5
# <<< typelet: codex-prompt-start"#;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub codex: bool,
    pub available: bool,
    pub config_path: String,
}

fn codex_config_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法找到用户主目录".to_owned())?;
    Ok(home.join(".codex").join("config.toml"))
}

fn read_config(path: &PathBuf) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("读取 Codex 配置失败：{error}")),
    }
}

fn contains_typelet_hook(config: &str) -> bool {
    config.contains(HOOK_START) && config.contains(HOOK_END)
}

fn remove_typelet_hook(config: &str) -> String {
    let Some(start) = config.find(HOOK_START) else {
        return config.to_owned();
    };
    let Some(end_offset) = config[start..].find(HOOK_END) else {
        return config.to_owned();
    };
    let end = start + end_offset + HOOK_END.len();
    let prefix = config[..start].trim_end_matches(['\n', '\r']);
    let suffix = config[end..].trim_start_matches(['\n', '\r']);
    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => String::new(),
        (true, false) => format!("{suffix}\n"),
        (false, true) => format!("{prefix}\n"),
        (false, false) => format!("{prefix}\n\n{suffix}"),
    }
}

fn insert_typelet_hook(config: &str) -> String {
    if contains_typelet_hook(config) {
        return config.to_owned();
    }
    let clean = remove_typelet_hook(config);
    let insertion = clean.find("[hooks.state]");
    let mut result = String::with_capacity(clean.len() + CODEX_HOOK.len() + 2);
    match insertion {
        Some(index) => {
            result.push_str(clean[..index].trim_end_matches(['\n', '\r']));
            result.push_str("\n\n");
            result.push_str(CODEX_HOOK);
            result.push_str("\n\n");
            result.push_str(clean[index..].trim_start_matches(['\n', '\r']));
        }
        None => {
            result.push_str(clean.trim_end_matches(['\n', '\r']));
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(CODEX_HOOK);
        }
    }
    result.push('\n');
    result
}

fn write_config(path: &PathBuf, config: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建 Codex 配置目录失败：{error}"))?;
    }
    let temporary = path.with_extension("toml.typelet.tmp");
    fs::write(&temporary, config).map_err(|error| format!("写入 Codex 配置失败：{error}"))?;
    if let Ok(metadata) = fs::metadata(path) {
        let _ = fs::set_permissions(&temporary, metadata.permissions());
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("替换 Codex 配置失败：{error}"));
    }
    Ok(())
}

fn current_config() -> Result<(PathBuf, String), String> {
    let path = codex_config_path()?;
    let config = read_config(&path)?;
    Ok((path, config))
}

fn config_state(path: &PathBuf, config: &str) -> AgentConfig {
    AgentConfig {
        codex: contains_typelet_hook(config),
        available: path.exists() || path.parent().is_some_and(|parent| parent.exists()),
        config_path: path.display().to_string(),
    }
}

#[tauri::command]
pub fn get_agent_config() -> Result<AgentConfig, String> {
    let (path, config) = current_config()?;
    Ok(config_state(&path, &config))
}

#[tauri::command]
pub fn set_codex_agent_enabled(enabled: bool) -> Result<AgentConfig, String> {
    let (path, config) = current_config()?;
    let next = if enabled {
        insert_typelet_hook(&config)
    } else {
        remove_typelet_hook(&config)
    };
    if next != config {
        write_config(&path, &next)?;
    }
    Ok(config_state(&path, &next))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_before_hook_state_and_is_idempotent() {
        let source = "model = \"test\"\n\n[hooks.state]\nvalue = true\n";
        let once = insert_typelet_hook(source);
        let twice = insert_typelet_hook(&once);
        assert_eq!(once, twice);
        assert!(once.find(HOOK_START).unwrap() < once.find("[hooks.state]").unwrap());
    }

    #[test]
    fn removes_only_the_typelet_block() {
        let source = format!("before\n\n{CODEX_HOOK}\n\nafter\n");
        assert_eq!(remove_typelet_hook(&source), "before\n\nafter\n");
    }
}
