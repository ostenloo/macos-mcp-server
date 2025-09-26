use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::protocol::ToolDescription;

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub app_name: String,
    pub description: String,
}

impl Tool {
    pub fn new(app_name: String) -> Self {
        let slug = slugify(&app_name);
        let name = format!("app.{slug}");
        let description =
            format!("Execute AppleScript commands in the {app_name} application context.");
        Self {
            name,
            app_name,
            description,
        }
    }

    pub fn description(&self) -> ToolDescription {
        ToolDescription {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "script": {
                        "type": "string",
                        "description": "AppleScript commands to execute inside a 'tell application' block"
                    }
                },
                "required": ["script"],
            })),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Vec<Tool>,
    lookup: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new(tools: Vec<Tool>) -> Self {
        let mut lookup = HashMap::new();
        for tool in &tools {
            lookup.insert(tool.name.clone(), tool.clone());
        }
        Self { tools, lookup }
    }

    pub fn descriptions(&self) -> Vec<ToolDescription> {
        self.tools.iter().map(|tool| tool.description()).collect()
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.lookup.get(name)
    }
}

pub fn load_tools(scripts_dir: &Path) -> anyhow::Result<ToolRegistry> {
    let mut names: BTreeSet<String> = BTreeSet::new();

    if scripts_dir.exists() {
        collect_app_names(scripts_dir, &mut names, &["pdf"])?;
        let text_dir = scripts_dir.join("text");
        if text_dir.exists() {
            collect_app_names(&text_dir, &mut names, &["txt"])?;
        }
    }

    let tools: Vec<Tool> = names.into_iter().map(Tool::new).collect();
    Ok(ToolRegistry::new(tools))
}

fn collect_app_names(
    dir: &Path,
    names: &mut BTreeSet<String>,
    extensions: &[&str],
) -> anyhow::Result<()> {
    for entry in dir.read_dir()? {
        let entry = entry?;
        let path: PathBuf = entry.path();
        if path.is_dir() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            if extensions
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(ext))
            {
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    names.insert(stem.to_string());
                }
            }
        }
    }
    Ok(())
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || matches!(ch, '-' | '_' | '.' | '/') {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    slug.trim_matches('-').to_string()
}
