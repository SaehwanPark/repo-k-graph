const USER_MANUAL: &str = include_str!("../../../docs/user-manual.md");

#[test]
fn user_manual_documents_mcp_client_config_examples() {
  let mcp_section = mcp_server_section();

  for heading in [
    "### Codex",
    "### Claude Code",
    "### ForgeCode",
    "### Antigravity",
  ] {
    assert!(
      mcp_section.contains(heading),
      "MCP section should include {heading}"
    );
  }

  assert_client_subsection_contains(
    "Codex",
    &[
      "codex mcp add rkg -- /absolute/path/to/rkg mcp serve",
      "~/.codex/config.toml",
      "command = \"/absolute/path/to/rkg\"",
      "args = [\"mcp\", \"serve\"]",
    ],
  );
  assert_client_subsection_contains(
    "Claude Code",
    &[
      "claude mcp add --transport stdio rkg -- /absolute/path/to/rkg mcp serve",
      ".mcp.json",
      "\"command\": \"/absolute/path/to/rkg\"",
      "\"args\": [\"mcp\", \"serve\"]",
    ],
  );
  assert_client_subsection_contains(
    "ForgeCode",
    &[
      "forge mcp import",
      "\"command\":\"/absolute/path/to/rkg\"",
      "\"args\":[\"mcp\",\"serve\"]",
      ".mcp.json",
      "\"command\": \"/absolute/path/to/rkg\"",
      "\"args\": [\"mcp\", \"serve\"]",
    ],
  );
  assert_client_subsection_contains(
    "Antigravity",
    &[
      "mcp_config.json",
      "\"command\": \"/absolute/path/to/rkg\"",
      "\"args\": [\"mcp\", \"serve\"]",
      "\"cwd\": \"/absolute/path/to/repository\"",
    ],
  );
}

fn mcp_server_section() -> &'static str {
  let start = USER_MANUAL
    .find("## MCP Server")
    .expect("user manual should have an MCP Server section");
  let end = USER_MANUAL[start..]
    .find("## Versioning")
    .map(|offset| start + offset)
    .expect("MCP Server section should end before Versioning");
  &USER_MANUAL[start..end]
}

fn client_subsection(client: &str) -> &'static str {
  let section = mcp_server_section();
  let heading = format!("### {client}");
  let start = section
    .find(&heading)
    .unwrap_or_else(|| panic!("MCP section should include {heading}"));
  let rest = &section[start..];
  let end = rest
    .find("\n### ")
    .filter(|offset| *offset > 0)
    .unwrap_or(rest.len());
  &rest[..end]
}

fn assert_client_subsection_contains(client: &str, expected_values: &[&str]) {
  let subsection = client_subsection(client);
  for expected in expected_values {
    assert!(
      subsection.contains(expected),
      "{client} MCP example should include {expected:?}"
    );
  }
}
