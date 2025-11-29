use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PromptTemplate {
    pub id: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(min = 1))]
    pub template: String,
    pub variables: Vec<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PromptTemplate {
    pub fn new(name: String, description: Option<String>, template: String) -> Self {
        let variables = Self::extract_variables(&template);
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            name,
            description,
            template,
            variables,
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }

    fn extract_variables(template: &str) -> Vec<String> {
        // Simple extraction of {{variable}} placeholders
        let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
        re.captures_iter(template)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    pub fn render(&self, context: &serde_json::Value) -> Result<String, String> {
        let mut result = self.template.clone();

        for var in &self.variables {
            if let Some(value) = context.get(var) {
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                result = result.replace(&format!("{{{{{}}}}}", var), &replacement);
            } else {
                return Err(format!("Missing variable: {}", var));
            }
        }

        Ok(result)
    }
}
