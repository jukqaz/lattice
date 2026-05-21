#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretFinding {
    pub pattern: String,
}

const SECRET_MARKERS: &[&str] = &[
    "sk-", "ghp_", "gho_", "ghu_", "ghs_", "ghr_", "ctx7sk_", "AIza", "AQ.",
];

pub fn has_secret_like_content(content: &str) -> bool {
    !find_secret_like_patterns(content).is_empty()
}

pub fn find_secret_like_patterns(content: &str) -> Vec<String> {
    let mut findings = Vec::new();
    for marker in SECRET_MARKERS {
        if content.contains(marker) {
            findings.push((*marker).to_string());
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::{find_secret_like_patterns, has_secret_like_content};

    #[test]
    fn detects_common_secret_like_patterns() {
        let content = format!(
            "openai = \"{}proj_fake_but_token_shaped\"\ngithub = \"{}fakebuttoken\"\ngoogle = \"{}Fake\"\n",
            ["s", "k-"].concat(),
            ["g", "hp_"].concat(),
            ["A", "Iza"].concat()
        );
        let findings = find_secret_like_patterns(&content);

        assert!(findings.contains(&"sk-".to_string()));
        assert!(findings.contains(&"ghp_".to_string()));
        assert!(findings.contains(&"AIza".to_string()));
        assert!(has_secret_like_content(&["ctx", "7sk_fake"].concat()));
        assert!(!has_secret_like_content("ordinary config"));
    }
}
