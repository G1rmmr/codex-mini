//! Local, explainable hints: only recommend entries returned by model/list.
use crate::state::string;
use serde_json::Value;

pub fn effort(instructions: &str) -> &'static str {
    let text = instructions.to_lowercase();
    if [
        "보안",
        "설계",
        "아키텍처",
        "복잡",
        "정확",
        "security",
        "architecture",
        "complex",
        "thorough",
    ]
    .iter()
    .any(|k| text.contains(k))
    {
        "high"
    } else if [
        "간단", "빠르", "빠른", "속도", "저렴", "요약", "simple", "fast", "cheap", "summary",
    ]
    .iter()
    .any(|k| text.contains(k))
    {
        "low"
    } else {
        "medium"
    }
}

pub fn model<'a>(models: &'a [Value], instructions: &str) -> Option<&'a Value> {
    if instructions.trim().is_empty() {
        return None;
    }
    let default = || {
        models
            .iter()
            .find(|m| m["isDefault"] == true)
            .or(models.first())
    };
    if effort(instructions) == "low" {
        models
            .iter()
            .find(|m| {
                let description =
                    format!("{} {}", string(m, "model"), string(m, "description")).to_lowercase();
                ["mini", "lightweight", "fast", "빠른"]
                    .iter()
                    .any(|k| description.contains(k))
            })
            .or_else(default)
    } else {
        default()
    }
}

pub fn supported_effort<'a>(model: &'a Value, instructions: &str) -> &'a str {
    let preferred = effort(instructions);
    let efforts = model["supportedReasoningEfforts"].as_array();
    if efforts.is_some_and(|xs| xs.iter().any(|x| string(x, "reasoningEffort") == preferred)) {
        return preferred;
    }
    let default = string(model, "defaultReasoningEffort");
    if efforts.is_some_and(|xs| xs.iter().any(|x| string(x, "reasoningEffort") == default)) {
        default
    } else {
        efforts
            .and_then(|xs| xs.first())
            .map(|x| string(x, "reasoningEffort"))
            .unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn hints_stay_within_the_catalog_and_respect_task_complexity() {
        let models = vec![
            json!({"model":"general","isDefault":true,"defaultReasoningEffort":"medium","supportedReasoningEfforts":[{"reasoningEffort":"medium"}]}),
            json!({"model":"mini-fast"}),
        ];
        assert_eq!(
            string(model(&models, "빠른 요약").unwrap(), "model"),
            "mini-fast"
        );
        assert_eq!(
            string(model(&models, "보안 설계").unwrap(), "model"),
            "general"
        );
        assert_eq!(supported_effort(&models[0], "보안 설계"), "medium");
        assert!(model(&models, " ").is_none());
        assert!(model(&[], "설계").is_none());
    }
}
