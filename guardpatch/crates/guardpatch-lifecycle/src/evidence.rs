use guardpatch_policy::Config;

pub struct EvidenceResult {
    pub kind: String,
    pub passed: bool,
    pub output: String,
}

pub struct EvidenceRunner<'a> {
    #[allow(dead_code)]
    config: &'a Config,
}

impl<'a> EvidenceRunner<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub fn run_tests(&self) -> anyhow::Result<EvidenceResult> {
        self.run_command("tests", self.detect_test_command())
    }

    pub fn run_typecheck(&self) -> anyhow::Result<EvidenceResult> {
        self.run_command("typecheck", self.detect_typecheck_command())
    }

    fn run_command(&self, kind: &str, cmd: &str) -> anyhow::Result<EvidenceResult> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(EvidenceResult { kind: kind.to_string(), passed: false, output: "No command configured.".to_string() });
        }

        let output = std::process::Command::new(parts[0])
            .args(&parts[1..])
            .output();

        match output {
            Ok(out) => {
                let passed = out.status.success();
                let text = if passed {
                    String::from_utf8_lossy(&out.stdout).to_string()
                } else {
                    String::from_utf8_lossy(&out.stderr).to_string()
                };
                Ok(EvidenceResult { kind: kind.to_string(), passed, output: text })
            }
            Err(e) => Ok(EvidenceResult {
                kind: kind.to_string(),
                passed: false,
                output: format!("Command failed to launch: {}", e),
            }),
        }
    }

    fn detect_test_command(&self) -> &str {
        if std::path::Path::new("Cargo.toml").exists() {
            "cargo test"
        } else if std::path::Path::new("package.json").exists() {
            "npm test"
        } else if std::path::Path::new("pyproject.toml").exists() {
            "pytest"
        } else {
            "make test"
        }
    }

    fn detect_typecheck_command(&self) -> &str {
        if std::path::Path::new("tsconfig.json").exists() {
            "tsc --noEmit"
        } else if std::path::Path::new("Cargo.toml").exists() {
            "cargo check"
        } else {
            "make typecheck"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guardpatch_policy::Config;

    #[test]
    fn test_evidence_runner_detects_commands() {
        let config = Config::default();
        let runner = EvidenceRunner::new(&config);
        // Just verify the runner instantiates correctly
        let _ = runner.detect_test_command();
        let _ = runner.detect_typecheck_command();
    }
}
