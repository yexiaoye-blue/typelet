use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::PathBuf;

const CODEX_START: &str = "# >>> typelet: codex-prompt-start";
const CODEX_END: &str = "# <<< typelet: codex-prompt-start";
const CODEX_HOOK: &str = r#"# >>> typelet: codex-prompt-start
[[hooks.UserPromptSubmit]]
matcher = "*"

[[hooks.UserPromptSubmit.hooks]]
type = "command"
command = "nohup typelet >/dev/null 2>&1 </dev/null &"
command_windows = "Start-Process typelet"
timeout = 5
# <<< typelet: codex-prompt-start"#;
const CURSOR_COMMAND: &str = "nohup typelet >/dev/null 2>&1 </dev/null &";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub supported: bool,
    pub config_path: String,
    pub format: String,
    pub event: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDetail {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub supported: bool,
    pub editable: bool,
    pub config_path: String,
    pub format: String,
    pub event: String,
    pub content: String,
    pub note: String,
}

fn home_file(parts: &[&str]) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法找到用户主目录".to_owned())?;
    Ok(parts.iter().fold(home, |path, part| path.join(part)))
}

fn codex_path() -> Result<PathBuf, String> {
    home_file(&[".codex", "config.toml"])
}
fn cursor_path() -> Result<PathBuf, String> {
    home_file(&[".cursor", "hooks.json"])
}

fn read_config(path: &PathBuf) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("读取配置失败：{error}")),
    }
}

fn write_config(path: &PathBuf, config: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建配置目录失败：{error}"))?;
    }
    let temporary = path.with_extension("typelet.tmp");
    fs::write(&temporary, config).map_err(|error| format!("写入配置失败：{error}"))?;
    if let Ok(metadata) = fs::metadata(path) {
        let _ = fs::set_permissions(&temporary, metadata.permissions());
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("替换配置失败：{error}"));
    }
    Ok(())
}

fn remove_codex_hook(config: &str) -> String {
    let Some(start) = config.find(CODEX_START) else {
        return config.to_owned();
    };
    let Some(end_offset) = config[start..].find(CODEX_END) else {
        return config.to_owned();
    };
    let end = start + end_offset + CODEX_END.len();
    let prefix = config[..start].trim_end_matches(['\n', '\r']);
    let suffix = config[end..].trim_start_matches(['\n', '\r']);
    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => String::new(),
        (true, false) => format!("{suffix}\n"),
        (false, true) => format!("{prefix}\n"),
        (false, false) => format!("{prefix}\n\n{suffix}"),
    }
}

fn insert_codex_hook(config: &str, block: &str) -> String {
    let clean = remove_codex_hook(config);
    let mut result = String::with_capacity(clean.len() + block.len() + 2);
    if let Some(index) = clean.find("[hooks.state]") {
        result.push_str(clean[..index].trim_end_matches(['\n', '\r']));
        result.push_str("\n\n");
        result.push_str(block.trim());
        result.push_str("\n\n");
        result.push_str(clean[index..].trim_start_matches(['\n', '\r']));
    } else {
        result.push_str(clean.trim_end_matches(['\n', '\r']));
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(block.trim());
    }
    result.push('\n');
    result
}

fn codex_enabled(config: &str) -> bool {
    config.contains(CODEX_START) && config.contains(CODEX_END)
}

fn codex_block_from_editor(content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    let body = trimmed
        .strip_prefix(CODEX_START)
        .and_then(|value| value.strip_suffix(CODEX_END))
        .unwrap_or(trimmed)
        .trim();
    if !body.contains("[[hooks.UserPromptSubmit]]")
        || !body.contains("[[hooks.UserPromptSubmit.hooks]]")
        || !body.contains("type = \"command\"")
        || !body.contains("typelet")
    {
        return Err("Codex 配置需要保留 UserPromptSubmit、command 和 typelet".to_owned());
    }
    Ok(format!("{CODEX_START}\n{body}\n{CODEX_END}"))
}

fn cursor_default_hook() -> Value {
    json!({ "command": CURSOR_COMMAND, "timeout": 5, "matcher": "*" })
}

fn cursor_is_typelet_hook(value: &Value) -> bool {
    value
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains("typelet"))
}

fn cursor_root(config: &str) -> Result<Value, String> {
    if config.trim().is_empty() {
        return Ok(json!({ "version": 1, "hooks": {} }));
    }
    serde_json::from_str(config).map_err(|error| format!("Cursor hooks.json 格式错误：{error}"))
}

fn cursor_hooks_mut(root: &mut Value) -> Result<&mut serde_json::Map<String, Value>, String> {
    let object = root
        .as_object_mut()
        .ok_or_else(|| "Cursor hooks.json 根节点必须是对象".to_owned())?;
    object.entry("version").or_insert_with(|| json!(1));
    let hooks = object.entry("hooks").or_insert_with(|| json!({}));
    hooks
        .as_object_mut()
        .ok_or_else(|| "Cursor hooks.json 的 hooks 必须是对象".to_owned())
}

fn cursor_enabled(config: &str) -> Result<bool, String> {
    let mut root = cursor_root(config)?;
    let hooks = cursor_hooks_mut(&mut root)?;
    Ok(hooks
        .get("beforeSubmitPrompt")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(cursor_is_typelet_hook)))
}

fn cursor_update(config: &str, enabled: bool, hook: Value) -> Result<String, String> {
    let mut root = cursor_root(config)?;
    let hooks = cursor_hooks_mut(&mut root)?;
    let list = hooks
        .entry("beforeSubmitPrompt")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| "Cursor beforeSubmitPrompt 必须是数组".to_owned())?;
    list.retain(|item| !cursor_is_typelet_hook(item));
    if enabled {
        list.push(hook);
    }
    serde_json::to_string_pretty(&root)
        .map(|value| format!("{value}\n"))
        .map_err(|error| format!("生成 Cursor 配置失败：{error}"))
}

fn cursor_hook_from_editor(content: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content)
        .map_err(|error| format!("Cursor hook JSON 格式错误：{error}"))?;
    if !value.is_object() || !cursor_is_typelet_hook(&value) {
        return Err("Cursor hook 需要是对象，并且 command 中保留 typelet".to_owned());
    }
    Ok(value)
}

fn cursor_editor_content(config: &str) -> Result<String, String> {
    let mut root = cursor_root(config)?;
    let hooks = cursor_hooks_mut(&mut root)?;
    let hook = hooks
        .get("beforeSubmitPrompt")
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find(|item| cursor_is_typelet_hook(item)))
        .cloned()
        .unwrap_or_else(cursor_default_hook);
    serde_json::to_string_pretty(&hook).map_err(|error| format!("生成 Cursor hook 失败：{error}"))
}

fn summary(
    id: &str,
    enabled: bool,
    supported: bool,
    path: String,
    format: &str,
    event: &str,
) -> AgentSummary {
    let name = match id {
        "codex" => "Codex",
        "cursor" => "Cursor",
        _ => "DeepSeek Harness (dsh)",
    };
    AgentSummary {
        id: id.to_owned(),
        name: name.to_owned(),
        enabled,
        supported,
        config_path: path,
        format: format.to_owned(),
        event: event.to_owned(),
    }
}

fn all_summaries() -> Result<Vec<AgentSummary>, String> {
    let codex = codex_path()?;
    let codex_config = read_config(&codex)?;
    let cursor = cursor_path()?;
    let cursor_config = read_config(&cursor)?;
    Ok(vec![
        summary(
            "codex",
            codex_enabled(&codex_config),
            true,
            codex.display().to_string(),
            "TOML",
            "UserPromptSubmit",
        ),
        summary(
            "cursor",
            cursor_enabled(&cursor_config)?,
            true,
            cursor.display().to_string(),
            "JSON",
            "beforeSubmitPrompt",
        ),
        summary("dsh", false, false, String::new(), "Plugin", "待确认"),
    ])
}

fn detail(id: &str) -> Result<AgentDetail, String> {
    let summary = all_summaries()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "未知 Agent".to_owned())?;
    match id {
        "codex" => {
            let config = read_config(&codex_path()?)?;
            let content = if let Some(start) = config.find(CODEX_START) {
                config[start..]
                    .split(CODEX_END)
                    .next()
                    .unwrap_or(CODEX_HOOK)
                    .to_owned()
                    + CODEX_END
            } else {
                CODEX_HOOK.to_owned()
            };
            Ok(AgentDetail {
                id: summary.id,
                name: summary.name,
                enabled: summary.enabled,
                supported: true,
                editable: true,
                config_path: summary.config_path,
                format: summary.format,
                event: summary.event,
                content,
                note: "只编辑 Typelet 自己的 hook 片段，其他 Codex 配置会保留。".to_owned(),
            })
        }
        "cursor" => {
            let config = read_config(&cursor_path()?)?;
            Ok(AgentDetail { id: summary.id, name: summary.name, enabled: summary.enabled, supported: true, editable: true, config_path: summary.config_path, format: summary.format, event: summary.event, content: cursor_editor_content(&config)?, note: "只编辑 Typelet 的 beforeSubmitPrompt hook，保存后 Cursor 会自动重新读取 hooks.json。".to_owned() })
        }
        _ => Ok(AgentDetail {
            id: summary.id,
            name: summary.name,
            enabled: false,
            supported: false,
            editable: false,
            config_path: String::new(),
            format: summary.format,
            event: summary.event,
            content: String::new(),
            note: "DeepSeek Harness 目前使用插件架构，尚未确认稳定的全局任务 hook 接口。"
                .to_owned(),
        }),
    }
}

#[tauri::command]
pub fn get_agents_config() -> Result<Vec<AgentSummary>, String> {
    all_summaries()
}

#[tauri::command]
pub fn set_agent_enabled(agent: String, enabled: bool) -> Result<Vec<AgentSummary>, String> {
    match agent.as_str() {
        "codex" => {
            let path = codex_path()?;
            let current = read_config(&path)?;
            let next = if enabled {
                insert_codex_hook(&current, CODEX_HOOK)
            } else {
                remove_codex_hook(&current)
            };
            if next != current {
                write_config(&path, &next)?;
            }
        }
        "cursor" => {
            let path = cursor_path()?;
            let current = read_config(&path)?;
            let next = cursor_update(&current, enabled, cursor_default_hook())?;
            if next != current {
                write_config(&path, &next)?;
            }
        }
        "dsh" => return Err("DeepSeek Harness 的稳定 hook 接口尚未确认".to_owned()),
        _ => return Err("未知 Agent".to_owned()),
    }
    all_summaries()
}

#[tauri::command]
pub fn get_agent_detail(agent: String) -> Result<AgentDetail, String> {
    detail(&agent)
}

#[tauri::command]
pub fn save_agent_detail(agent: String, content: String) -> Result<AgentDetail, String> {
    match agent.as_str() {
        "codex" => {
            let path = codex_path()?;
            let current = read_config(&path)?;
            let block = codex_block_from_editor(&content)?;
            write_config(&path, &insert_codex_hook(&current, &block))?;
        }
        "cursor" => {
            let path = cursor_path()?;
            let current = read_config(&path)?;
            let hook = cursor_hook_from_editor(&content)?;
            write_config(&path, &cursor_update(&current, true, hook)?)?;
        }
        "dsh" => return Err("DeepSeek Harness 的稳定 hook 接口尚未确认".to_owned()),
        _ => return Err("未知 Agent".to_owned()),
    }
    detail(&agent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_and_cursor_updates_preserve_other_hooks() {
        let source = "model = \"test\"\n\n[hooks.state]\nvalue = true\n";
        let once = insert_codex_hook(source, CODEX_HOOK);
        assert!(once.find(CODEX_START).unwrap() < once.find("[hooks.state]").unwrap());
        assert!(insert_codex_hook(&once, CODEX_HOOK).contains(CODEX_START));

        let cursor = r#"{"version":1,"hooks":{"stop":[{"command":"echo stop"}]}}"#;
        let next = cursor_update(cursor, true, cursor_default_hook()).unwrap();
        assert!(next.contains("beforeSubmitPrompt"));
        assert!(next.contains("echo stop"));
        let removed = cursor_update(&next, false, cursor_default_hook()).unwrap();
        assert!(!removed.contains("nohup typelet"));
        assert!(removed.contains("echo stop"));
    }

    #[test]
    fn editor_content_keeps_owned_hook_boundaries() {
        let edited = codex_block_from_editor("[[hooks.UserPromptSubmit]]\n[[hooks.UserPromptSubmit.hooks]]\ntype = \"command\"\ncommand = \"typelet --summon\"").unwrap();
        assert!(edited.starts_with(CODEX_START));
        assert!(edited.ends_with(CODEX_END));
    }
}
