use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[default]
    #[serde(rename = "ko", other)]
    Korean,
}

impl Language {
    pub fn text(self, key: &str) -> &str {
        if self == Self::English {
            CATALOG
                .iter()
                .find(|(ko, _)| *ko == key)
                .map_or(key, |(_, en)| *en)
        } else {
            key
        }
    }

    pub fn format(self, key: &str, args: &[&dyn Display]) -> String {
        let mut rest = self.text(key);
        let mut out = String::new();
        for arg in args {
            if let Some((prefix, tail)) = rest.split_once('{')
                && let Some((_, suffix)) = tail.split_once('}')
            {
                out.push_str(prefix);
                out.push_str(&arg.to_string());
                rest = suffix;
            }
        }
        out.push_str(rest);
        out
    }

    // Only app-owned status/error strings enter here. Never translate prompts,
    // replies, tool payloads, paths, names or approval content with this method.
    pub fn message(self, source: &str) -> Cow<'_, str> {
        if self == Self::Korean {
            return Cow::Borrowed(source);
        }
        let exact = self.text(source);
        if exact != source {
            return Cow::Borrowed(exact);
        }
        for (ko, _) in CATALOG {
            let Some((prefix, tail)) = ko.split_once('{') else {
                continue;
            };
            let Some((_, suffix)) = tail.split_once('}') else {
                continue;
            };
            if suffix.contains('{') || prefix.is_empty() {
                continue;
            }
            if let Some(inner) = source
                .strip_prefix(prefix)
                .and_then(|s| s.strip_suffix(suffix))
            {
                return Cow::Owned(self.format(ko, &[&inner]));
            }
        }
        Cow::Borrowed(source)
    }
}

const CATALOG: &[(&str, &str)] = &[
    ("화면", "Appearance"),
    ("기본 · RTK", "Default · RTK"),
    ("{}\n… [화면 표시 생략]", "{}\n… [display truncated]"),
    ("나", "YOU"),
    ("명령 · {status}", "Command · {status}"),
    ("파일 변경 · {status}", "File changes · {status}"),
    ("도구 · {status}", "Tool · {status}"),
    ("웹 검색", "Web search"),
    ("계획", "Plan"),
    (
        "프로젝트는 최대 32개까지 저장할 수 있습니다.",
        "Up to 32 projects can be saved.",
    ),
    (
        "프로젝트 이름은 1~120바이트로 입력하세요.",
        "Project names must be 1–120 bytes.",
    ),
    (
        "경로는 4 KiB, 지침은 16 KiB 이하로 입력하세요.",
        "Paths must fit within 4 KiB; instructions within 16 KiB.",
    ),
    ("프로젝트", "Projects"),
    (
        "설정 저장 경로가 없습니다.",
        "Settings location unavailable.",
    ),
    ("사용량 저장 실패: {e}", "Could not save usage: {e}"),
    ("사용량 저장 종료됨", "Usage storage stopped"),
    (
        "사용량 저장 대기열 오류: {e}",
        "Usage storage queue error: {e}",
    ),
    (
        "설정 파일이 1 MiB를 넘습니다.",
        "Settings file exceeds 1 MiB.",
    ),
    (
        "설정 파일은 1 MiB 이하로 저장하세요.",
        "Keep the settings file under 1 MiB.",
    ),
    ("TOML 문법 오류: {e}", "TOML syntax error: {e}"),
    (
        "다른 프로그램에서 파일을 변경했습니다. 다시 불러온 뒤 수정하세요.",
        "Another program changed this file. Reload it before editing.",
    ),
    ("설정 경로 오류", "Invalid settings path"),
    (
        "저장 직전에 파일이 변경되었습니다. 다시 불러오세요.",
        "The file changed before saving. Reload it.",
    ),
    (
        "Codex 설정 폴더를 찾을 수 없습니다.",
        "Codex settings folder not found.",
    ),
    (
        "저장했습니다. 연결을 종료한 뒤 다시 연결하면 적용됩니다.",
        "Saved. Disconnect and reconnect to apply.",
    ),
    ("전역 설정", "Global settings"),
    ("공통 지침", "Instructions"),
    ("모델·권한", "Model / access"),
    ("스킬", "Skills"),
    ("실행·MCP", "CLI / MCP"),
    ("고급", "Advanced"),
    ("설정 처리 중…", "Loading settings…"),
    ("다시 불러오기", "Reload"),
    (
        "config.toml 문법 오류가 있습니다. 고급 탭에서 수정하세요.",
        "Invalid config.toml. Fix it in Advanced.",
    ),
    (
        "모든 프로젝트에 적용되는 공통 지침",
        "Shared instructions for all projects",
    ),
    ("공통 지침 저장", "Save instructions"),
    (
        "Codex 사용자 기본값 · 프로젝트 설정이나 관리 정책이 우선할 수 있습니다.",
        "Codex user defaults. Project settings or managed policies may take precedence.",
    ),
    ("기본 모델", "Default model"),
    ("추론 강도", "Reasoning effort"),
    ("승인 정책", "Approval policy"),
    ("실행 권한", "Sandbox mode"),
    ("Codex 기본값", "Codex default"),
    (
        "read-only: 읽기 전용 · workspace-write: 작업 폴더 편집 · danger-full-access: 제한 없음",
        "read-only: read files · workspace-write: edit project files · danger-full-access: unrestricted",
    ),
    (
        "untrusted: 신뢰되지 않은 작업 확인 · on-request: 필요할 때 확인 · never: 승인 질문 없음",
        "untrusted: confirm untrusted actions · on-request: confirm when needed · never: no approval prompts",
    ),
    ("기본값 저장", "Save defaults"),
    (
        "Codex가 인식하는 스킬 · 켜기/끄기는 전역 사용자 설정에 저장됩니다.",
        "Skills recognized by Codex. Changes are saved in global user settings.",
    ),
    (
        "먼저 메인 화면에서 Codex에 연결하세요.",
        "Connect to Codex from the main window first.",
    ),
    ("스킬 목록 새로고침", "Refresh skills"),
    ("스킬 검색", "Search skills"),
    (
        "스킬을 변경하려면 다른 전역 설정을 먼저 저장하세요.",
        "Save other global settings before changing skills.",
    ),
    ("Codex 실행 파일", "Codex executable"),
    ("실행 경로 저장", "Save CLI path"),
    (
        "실행 경로는 Mini에 저장됩니다. MCP 설정은 Codex config.toml에 저장됩니다.",
        "The CLI path is saved in Mini. MCP settings are saved in Codex config.toml.",
    ),
    ("MCP 서버", "MCP servers"),
    ("편집", "Edit"),
    ("서버 추가 / 편집", "Add / edit server"),
    ("서버 이름", "Server name"),
    ("로컬 명령", "Local command"),
    ("서버 URL", "Server URL"),
    ("실행 파일", "Executable"),
    ("인수 · 한 줄에 하나", "Arguments · one per line"),
    ("서버 설정 반영", "Apply server settings"),
    (
        "아래 저장 버튼을 눌러 적용하세요.",
        "Click Save below to persist these changes.",
    ),
    ("MCP 설정 저장", "Save MCP settings"),
    (
        "환경 변수·인증·프로필 등 추가 옵션은 고급 탭에서 편집할 수 있습니다.",
        "Edit environment variables, authentication and profiles in Advanced.",
    ),
    ("config.toml 저장", "Save config.toml"),
    (
        "작업이 끝난 뒤 전역 설정을 변경할 수 있습니다.",
        "Global settings can be changed after the task finishes.",
    ),
    ("파일 다시 불러오기", "Reload files"),
    (
        "파일 저장 전 원본을 백업하며 외부 변경을 감지하면 저장을 중단합니다.",
        "Files are backed up before saving. External changes prevent overwrites.",
    ),
    (
        "서버 이름은 영문·숫자·밑줄·하이픈 1~64자로 입력하세요.",
        "Use 1–64 ASCII letters, digits, underscores or hyphens for server names.",
    ),
    (
        "실행 파일 또는 http(s) URL을 입력하세요.",
        "Enter an executable or an http(s) URL.",
    ),
    (
        "mcp_servers가 테이블 형식이 아닙니다.",
        "mcp_servers must be a table.",
    ),
    (
        "서버 설정이 테이블 형식이 아닙니다.",
        "Server settings must be a table.",
    ),
    ("Codex 전송 실패: {e}", "Could not send to Codex: {e}"),
    ("Codex 응답 형식 오류: {e}", "Invalid Codex response: {e}"),
    ("Codex 연결 오류: {e}", "Codex connection error: {e}"),
    ("Codex 연결이 종료되었습니다.", "Codex disconnected."),
    ("연결이 종료되었습니다.", "Disconnected."),
    ("전송할 수 없습니다: {e}", "Could not send: {e}"),
    ("기본", "Default"),
    (
        "Windows에서는 RTK 자동 필터를 꺼 주세요. 현재 Bash 명령만 지원합니다.",
        "Disable RTK filtering on Windows. Only Bash commands are supported.",
    ),
    (
        "RTK 실행 파일이 없습니다. 로그 필터를 끄고 다시 시도하세요.",
        "RTK executable not found. Disable log filtering and retry.",
    ),
    ("명령 출력 필터 확인", "Command output filter check"),
    ("연결 전 · 대기 중", "Offline · idle"),
    ("Codex에 먼저 연결하세요.", "Connect to Codex first."),
    ("설정 저장 실패: {e}", "Could not save settings: {e}"),
    (
        "존재하는 작업 폴더의 경로를 입력하세요.",
        "Enter an existing project folder.",
    ),
    ("Codex 연결 중…", "Connecting to Codex…"),
    (
        "Codex를 실행하지 못했습니다: {e}\n터미널에서 codex login을 먼저 실행하세요.",
        "Could not start Codex: {e}\nRun codex login in a terminal first.",
    ),
    (
        "연결 종료 · Codex 프로세스 해제",
        "Disconnected · Codex stopped",
    ),
    ("새 대화", "New chat"),
    (
        "메시지는 128 KiB 이하로 입력하세요.",
        "Keep messages under 128 KiB.",
    ),
    ("대화 준비 중…", "Preparing chat…"),
    ("작업 중", "Working"),
    ("프로젝트 전환 · 연결 전", "Project changed · offline"),
    ("이전 대화 불러오는 중…", "Loading previous chat…"),
    ("추가", "Add"),
    ("프로젝트 편집", "Edit project"),
    ("연결 후 대화를 열 수 있습니다.", "Connect to open a chat."),
    ("최근 대화 · 최대 20개", "Recent chats · up to 20"),
    ("프로젝트 추가", "Add project"),
    ("이름", "Name"),
    ("작업 폴더", "Folder"),
    ("프로젝트 지침", "Project instructions"),
    (
        "이 프로젝트에서 공통으로 지킬 작업 방식",
        "Instructions shared by tasks in this project",
    ),
    (
        "지침은 이 프로젝트의 새 대화·이어쓰기에 적용됩니다.",
        "Applied to new and resumed chats in this project.",
    ),
    ("저장", "Save"),
    (
        "이름은 1~120바이트, 지침은 16 KiB 이하로 입력하세요.",
        "Names must be 1–120 bytes; instructions must fit within 16 KiB.",
    ),
    (
        "존재하는 작업 폴더를 입력하세요.",
        "Enter an existing folder.",
    ),
    ("작업 폴더를 선택하세요.", "Select a folder."),
    ("프로젝트를 찾을 수 없습니다.", "Project not found."),
    (
        "스킬 설정 요청 실패: {}",
        "Could not change skill settings: {}",
    ),
    (
        "요청 실패 · 내용을 확인하세요",
        "Request failed · check details",
    ),
    ("전역 스킬이 없습니다.", "No global skills found."),
    (
        "스킬 설정을 저장했습니다. 다시 연결하면 적용됩니다.",
        "Skill settings saved. Reconnect to apply.",
    ),
    ("연결됨 · 대기 중", "Connected · idle"),
    ("로그인 필요", "Login required"),
    ("로컬 설정 사용", "Using local settings"),
    (
        "Codex가 대화 ID를 반환하지 않았습니다.",
        "Codex returned no chat ID.",
    ),
    ("대화를 이어갈 수 있습니다", "Chat ready to resume"),
    ("중단 요청됨…", "Stopping…"),
    ("완료 · {elapsed}초", "Done · {elapsed}s"),
    ("작업 중단됨", "Task stopped"),
    ("작업 실패", "Task failed"),
    ("지원하지 않는 요청", "Unsupported request"),
    (
        "{method}\n요청을 실행하지 않았습니다. 필요한 경우 공식 Codex CLI에서 이어가세요.",
        "{method}\nRequest skipped. Continue in the official Codex CLI if needed.",
    ),
    ("확인이 필요합니다", "Approval required"),
    ("답변 직접 입력", "Enter an answer"),
    (
        "아래 작업을 검토한 뒤 결정하세요.",
        "Review the action before deciding.",
    ),
    (
        "전체 내용을 표시할 수 없어 허용을 비활성화했습니다. CLI에서 검토하세요.",
        "Full details could not be displayed. Approval is disabled. Review in the CLI.",
    ),
    ("답변 보내기", "Send answers"),
    (
        "요청한 권한 허용 · 이번 작업만",
        "Allow permissions · this turn",
    ),
    ("거절", "Decline"),
    ("이번 한 번 허용", "Allow once"),
    (
        "작업 중단은 메인 창에서 할 수 있습니다.",
        "Stop the task in the main window.",
    ),
    ("사용량·절약", "Usage / savings"),
    ("현재 작업", "Current task"),
    ("서버 사용량 수신 대기 중", "Waiting for server usage"),
    (
        "{} · {} · 도구 {}회 · {}초",
        "{} · {} · {} tool calls · {}s",
    ),
    (
        "복원 후 관측분만 포함합니다. 첫 호출 이전 사용량은 제외됩니다.",
        "Only usage observed since resuming is included. Earlier usage is excluded.",
    ),
    ("아직 측정한 작업이 없습니다.", "No measured tasks yet."),
    (
        "캐시는 입력에, 추론은 출력에 포함됩니다. 절감률·요금 추정값이 아닙니다.",
        "Cached tokens are part of input; reasoning is part of output. These are not savings or cost estimates.",
    ),
    (
        "이 프로젝트 · 다음 새 대화부터",
        "This project · new chats only",
    ),
    (
        "기존 대화는 시작할 때 선택한 설정을 유지합니다.",
        "Existing chats keep their original settings.",
    ),
    ("기본 추론", "Default effort"),
    (
        "환경 설정에서 지정한 추론 값이 우선합니다. 모델별 지원 범위가 다릅니다.",
        "Settings override this effort. Supported values depend on the model.",
    ),
    ("Cargo 로그 필터 (RTK)", "Cargo log filter (RTK)"),
    (
        "test / clippy / build / check만 필터링. 생략된 진단은 RTK 복구 힌트를 확인하세요.",
        "Filters test / clippy / build / check. Use RTK recovery hints for omitted diagnostics.",
    ),
    (
        "RTK 자동 필터는 Linux·macOS의 Bash 명령에서 지원됩니다.",
        "Automatic RTK filtering supports Bash on Linux and macOS.",
    ),
    (
        "RTK 미설치 · scripts/install-rtk.sh 실행 후 앱을 다시 여세요.",
        "RTK not installed. Run scripts/install-rtk.sh, then reopen Mini.",
    ),
    ("스킬·MCP 제외", "Exclude skills / MCP"),
    (
        "필요 없는 항목만 선택하세요. 공유 Codex 설정은 바뀌지 않습니다.",
        "Exclude only what you don't need. Shared Codex settings stay unchanged.",
    ),
    ("스킬 목록 불러오기", "Load skills"),
    ("제외: {}{}", "Exclude: {}{}"),
    (" (이미 꺼짐)", " (already disabled)"),
    (
        "MCP 제외 이름 · 쉼표 구분",
        "Excluded MCP names · comma separated",
    ),
    ("MCP 제외 적용", "Apply MCP exclusions"),
    (
        "MCP 이름은 영문·숫자·밑줄·하이픈만 사용할 수 있습니다.",
        "MCP names may contain only ASCII letters, digits, underscores and hyphens.",
    ),
    ("제외 항목 초기화", "Reset exclusions"),
    (
        "스킬 {}개 · MCP {}개 제외",
        "{} skills · {} MCP servers excluded",
    ),
    ("최근 작업 · {} / {}", "Recent tasks · {} / {}"),
    ("JSON 복사", "Copy JSON"),
    (
        "이 PC의 최근 200개 작업. 프롬프트·코드·응답 본문은 기록하지 않습니다.",
        "Last 200 tasks on this PC. Prompts, code and response bodies are not logged.",
    ),
    ("{} · {} · {} · {}초", "{} · {} · {} · {}s"),
    ("서버 사용량 없음", "No server usage"),
    ("{} · 도구 {}회", "{} · {} tool calls"),
    ("복원 후 관측분", "Observed since resuming"),
    ("모델", "Model"),
    ("추론", "Effort"),
    ("기본값", "Default"),
    (
        "추가 권한이 필요하면 승인을 요청합니다.",
        "Extra permissions require approval.",
    ),
    ("설정", "Settings"),
    ("대화", "Chats"),
    ("최근 대화 없음", "No recent chats"),
    ("대화 ID 복사", "Copy chat ID"),
    ("연결", "Connect"),
    ("연결 종료", "Disconnect"),
    ("닫기", "Dismiss"),
    ("메시지 입력", "Message"),
    ("Ctrl+Enter 전송", "Ctrl+Enter to send"),
    ("보내기", "Send"),
    (
        "연결 후 메시지를 입력하세요.",
        "Connect, then enter a message.",
    ),
    ("중단", "Stop"),
    ("대화 없음", "No conversation"),
    ("아래에 작업 내용을 입력하세요.", "Enter a task below."),
    (
        "이전 내용 일부는 화면에서 생략했습니다. 원본은 Codex에 남아 있습니다.",
        "Some earlier content is hidden here. The original remains in Codex.",
    ),
    ("복사", "Copy"),
    ("프로젝트 목록", "Project list"),
    (
        "실행 경로를 저장했습니다. 다음 연결부터 적용됩니다.",
        "CLI path saved. Applies on the next connection.",
    ),
    ("환경 설정", "Settings"),
    (
        "전역 설정 · 지침 / 스킬 / MCP",
        "Global settings · instructions / skills / MCP",
    ),
    ("변경 적용 · 다시 연결", "Apply changes · reconnect"),
    ("ChatGPT 로그인", "Log in to ChatGPT"),
    ("진단", "Diagnostics"),
    ("로그인", "Login"),
    (
        "브라우저에서 ChatGPT 로그인 열기",
        "Open ChatGPT login in browser",
    ),
    ("연결 전", "Offline"),
    ("Codex 실행 파일: {}", "Codex executable: {}"),
    (
        "로그는 최대 40줄만 메모리에 보관합니다.",
        "Only the last 40 log lines are kept in memory.",
    ),
    ("입력", "Input"),
    ("캐시 입력 (포함)", "Cached input (included)"),
    ("출력", "Output"),
    ("추론 출력 (포함)", "Reasoning output (included)"),
    ("실행할 명령", "Command"),
    ("요청 이유", "Reason"),
    ("허용할 경로", "Allowed path"),
    ("네트워크 접근", "Network access"),
    ("요청한 권한", "Requested permissions"),
    ("추가 권한", "Additional permissions"),
    ("작업 내용\n{previous}", "Action details\n{previous}"),
    ("[화면 표시 생략]", "[display truncated]"),
    ("언어", "Language"),
    ("테마", "Theme"),
    ("라이트", "Light"),
    ("다크", "Dark"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_preserves_arguments_and_external_text() {
        let mut seen = std::collections::HashSet::new();
        for (ko, en) in CATALOG {
            assert!(seen.insert(ko), "duplicate key: {ko}");
            assert_eq!(ko.matches('{').count(), en.matches('{').count(), "{ko}");
        }
        let external = "한글 경로/{elapsed}/project";
        assert_eq!(
            Language::English.format("Codex 실행 파일: {}", &[&external]),
            format!("Codex executable: {external}")
        );
        assert_eq!(Language::English.message("완료 · 15초"), "Done · 15s");
        assert_eq!(
            Language::English.message("Codex 연결 오류: 원본 오류 {e}"),
            "Codex connection error: 원본 오류 {e}"
        );
        assert_eq!(Language::Korean.message("완료 · 15초"), "완료 · 15초");
        assert_eq!(Language::English.message(external), external);
    }

    #[test]
    fn preferences_roundtrip_and_old_settings_keep_their_defaults() {
        use crate::{preferences::Theme, state::Settings};
        let mut settings: Settings =
            serde_json::from_str(r#"{"cwd":"/project","model":"saved-model"}"#).unwrap();
        assert_eq!(settings.language, Language::Korean);
        assert_eq!(settings.theme, Theme::Dark);
        settings.language = Language::English;
        settings.theme = Theme::Light;
        let saved = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.language, Language::English);
        assert_eq!(loaded.theme, Theme::Light);
        assert_eq!(loaded.model, "saved-model");
        let future: Settings =
            serde_json::from_str(r#"{"cwd":"/project","language":"unknown","theme":"unknown"}"#)
                .unwrap();
        assert_eq!(future.cwd, "/project");
        assert_eq!(future.language, Language::Korean);
    }
}
