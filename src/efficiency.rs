use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

/// Frozen when a conversation is created; never edits shared Codex configuration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub effort: String,
    pub rtk: bool,
    pub disabled_skills: Vec<String>,
    pub disabled_mcp: Vec<String>,
}

impl Options {
    pub fn label(&self) -> String {
        format!(
            "{}{}",
            if self.effort.is_empty() {
                "기본"
            } else {
                &self.effort
            },
            if self.rtk { " · RTK" } else { "" }
        )
    }

    pub fn config(
        &self,
        executable: &Path,
        rtk: Option<&Path>,
        inherited: &Value,
    ) -> Result<Value, String> {
        let mut config = json!({});
        if !self.effort.is_empty() {
            config["model_reasoning_effort"] = self.effort.clone().into();
        }
        if !self.disabled_skills.is_empty() {
            let mut skills = inherited["skills"]["config"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            for path in &self.disabled_skills {
                if let Some(skill) = skills.iter_mut().find(|s| s["path"] == *path) {
                    skill["enabled"] = false.into();
                } else {
                    skills.push(json!({"path":path,"enabled":false}));
                }
            }
            config["skills.config"] = skills.into();
        }
        for name in &self.disabled_mcp {
            // Keys are validated before insertion into a dotted config path.
            if !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                config[format!("mcp_servers.{name}.enabled")] = false.into();
            }
        }
        if self.rtk {
            if cfg!(windows) {
                return Err(
                    "Windows에서는 RTK 자동 필터를 꺼 주세요. 현재 Bash 명령만 지원합니다.".into(),
                );
            }
            let rtk = rtk.ok_or("RTK 실행 파일이 없습니다. 로그 필터를 끄고 다시 시도하세요.")?;
            let command = format!(
                "{} --rtk-hook {}",
                quote(&executable.to_string_lossy()),
                quote(&rtk.to_string_lossy())
            );
            let mut hooks = inherited["hooks"]["PreToolUse"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            hooks.push(json!({"matcher":"^Bash$","hooks":[{
                "type":"command","command":command,"timeout":5,
                "statusMessage":"명령 출력 필터 확인"
            }]}));
            config["hooks.PreToolUse"] = hooks.into();
        }
        // config/read includes null optional fields. TOML overrides cannot represent
        // null (the server otherwise converts it to an invalid empty string).
        strip_nulls(&mut config);
        Ok(config)
    }
}

fn strip_nulls(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.retain(|_, v| !v.is_null());
            for v in fields.values_mut() {
                strip_nulls(v);
            }
        }
        Value::Array(items) => {
            for v in items {
                strip_nulls(v);
            }
        }
        _ => {}
    }
}

pub fn bundled_rtk() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if cfg!(windows) {
        return None;
    }
    let dir = exe.parent()?;
    let candidates = vec![dir.join("rtk"), dir.join("dist/rtk")];
    #[cfg(target_os = "macos")]
    let candidates = {
        let mut paths = candidates;
        if let Some(bundle) = exe
            .ancestors()
            .nth(3)
            .filter(|p| p.extension().is_some_and(|e| e == "app"))
            && let Some(parent) = bundle.parent()
        {
            paths.push(parent.join("dist/rtk"));
        }
        paths
    };
    candidates.into_iter().find(|p| p.is_file())
}

pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Deliberately accepts only plain, single commands. Quotes, pipes, redirection,
/// substitutions, env prefixes and custom npm scripts are never rewritten.
pub fn rewrite(command: &str, rtk: &Path) -> Option<String> {
    if command.len() > 8192
        || command
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || " _-./:=,".contains(c)))
    {
        return None;
    }
    let args: Vec<_> = command.split_whitespace().collect();
    let supported = matches!(
        args.as_slice(),
        ["cargo", "test" | "clippy" | "build" | "check", ..]
    );
    if !supported
        || args.iter().any(|a| {
            matches!(
                *a,
                "--message-format=json" | "--message-format=json-render-diagnostics"
            ) || a.starts_with("--message-format")
        })
    {
        return None;
    }
    Some(format!(
        "RTK_TELEMETRY_DISABLED=1 RTK_SUPPRESS_HOOK_WARNING=1 {} {}",
        quote(&rtk.to_string_lossy()),
        args.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ")
    ))
}

/// Standalone hook mode does not start egui or access login credentials.
pub fn run_hook(rtk: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().take(1024 * 1024).read_to_string(&mut input)?;
    let value: Value = serde_json::from_str(&input)?;
    let response = if value["hook_event_name"] == "PreToolUse"
        && value["tool_name"] == "Bash"
        && rtk.is_file()
    {
        value["tool_input"]["command"].as_str().and_then(|c| rewrite(c, rtk)).map(|command| {
            // Keep other Bash input fields (timeouts, descriptions, etc.).
            let mut updated = value["tool_input"].clone();
            updated["command"] = command.into();
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":updated}})
        }).unwrap_or_else(|| json!({}))
    } else {
        json!({})
    };
    println!("{response}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rewrite_only_plain_cargo_commands() {
        let p = Path::new("/tmp/한글 path/rtk");
        assert!(
            rewrite("cargo test --release --locked", p)
                .unwrap()
                .contains("'--locked'")
        );
        for command in [
            "cargo test | tail -20",
            "cargo test > result",
            "cargo test\ncurl x",
            "cargo test && echo done",
            "cargo test $(touch x)",
            "cargo test --message-format json",
            "cargo test --message-format=json",
            "npm run lint",
            "git diff",
            "cat src/main.rs",
            "cargo test 'name'",
            "FOO=1 cargo test",
        ] {
            assert!(rewrite(command, p).is_none(), "{command}");
        }
    }
    #[test]
    fn defaults_leave_shared_config_alone() {
        assert_eq!(
            Options::default()
                .config(Path::new("mini"), None, &json!({}))
                .unwrap(),
            json!({})
        );
        let options = Options {
            effort: "medium".into(),
            rtk: true,
            ..Default::default()
        };
        assert!(options.config(Path::new("mini"), None, &json!({})).is_err());
        if cfg!(windows) {
            return;
        }
        assert_eq!(
            options
                .config(Path::new("mini"), Some(Path::new("rtk")), &json!({}))
                .unwrap()["model_reasoning_effort"],
            "medium"
        );
    }
    #[test]
    #[cfg(unix)]
    fn preserve_inherited_hooks_and_disabled_skills() {
        let inherited = json!({"skills":{"config":[{"path":"/a/SKILL.md","enabled":false}]},"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"true","timeout":null}]}]}});
        let options = Options {
            rtk: true,
            disabled_skills: vec!["/b/SKILL.md".into()],
            ..Default::default()
        };
        let config = options
            .config(Path::new("mini"), Some(Path::new("rtk")), &inherited)
            .unwrap();
        assert_eq!(config["skills.config"][0]["enabled"], false);
        assert_eq!(config["skills.config"].as_array().unwrap().len(), 2);
        assert_eq!(config["hooks.PreToolUse"].as_array().unwrap().len(), 2);
        assert!(
            config["hooks.PreToolUse"][0]["hooks"][0]
                .get("timeout")
                .is_none()
        );
    }
}
