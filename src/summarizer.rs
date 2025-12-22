//! GPT-4.1 summarization module for crate documentation.

use std::time::Duration;

use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use async_openai::Client;

use crate::error::SummaryError;
use crate::parser::CrateDocs;

/// The system prompt for the summarizer.
const SYSTEM_PROMPT: &str = r#"You are a technical documentation summarizer for Rust crates. Your task is to create concise, actionable summaries that help developers understand how to use a crate effectively.

Your summary should focus on:
1. What the crate does (one clear sentence)
2. Core types and their purposes
3. Key patterns for common use cases
4. Important traits to implement or use
5. Getting started guidance

Guidelines:
- Keep summaries factual and based only on the provided documentation
- Use clear, concise language suitable for experienced Rust developers
- Highlight the most important types and functions
- Include brief code snippets only if they clarify usage
- Output in Markdown format with clear sections

Do not include:
- Version numbers or changelog information
- Installation instructions (users know cargo add)
- Verbose explanations of basic Rust concepts
- Speculation about features not documented"#;

/// Client for summarizing crate documentation using GPT-4.1.
pub struct Summarizer {
    client: Client<OpenAIConfig>,
    model: String,
}

impl Summarizer {
    /// Creates a new summarizer using the `OPENAI_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns `SummaryError::MissingApiKey` if the environment variable is not set.
    pub fn from_env() -> Result<Self, SummaryError> {
        if std::env::var("OPENAI_API_KEY").is_err() {
            return Err(SummaryError::MissingApiKey);
        }

        let config = OpenAIConfig::default();
        let client = Client::with_config(config);

        Ok(Self {
            client,
            model: "gpt-4.1".to_owned(),
        })
    }

    /// Creates a new summarizer with a custom model name.
    ///
    /// # Errors
    ///
    /// Returns `SummaryError::MissingApiKey` if the API key is not set.
    pub fn with_model(model: impl Into<String>) -> Result<Self, SummaryError> {
        let mut summarizer = Self::from_env()?;
        summarizer.model = model.into();
        Ok(summarizer)
    }

    /// Summarizes crate documentation.
    ///
    /// # Errors
    ///
    /// Returns `SummaryError` if the API call fails or rate limits are hit.
    pub async fn summarize(&self, docs: &CrateDocs) -> Result<String, SummaryError> {
        let docs_text = crate::parser::format_docs_summary(docs);
        let user_prompt = format_user_prompt(&docs.index.name, &docs.index.version, &docs_text);

        self.call_with_retry(&user_prompt, 3).await
    }

    /// Makes an API call with retry logic for rate limiting.
    async fn call_with_retry(
        &self,
        user_prompt: &str,
        max_retries: u32,
    ) -> Result<String, SummaryError> {
        let mut last_error = None;
        let mut delay = Duration::from_secs(1);

        for attempt in 0..=max_retries {
            if attempt > 0 {
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }

            match self.make_request(user_prompt).await {
                Ok(response) => return Ok(response),
                Err(SummaryError::RateLimited { retry_after_secs }) => {
                    delay = Duration::from_secs(retry_after_secs);
                    last_error = Some(SummaryError::RateLimited { retry_after_secs });
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SummaryError::ApiError("Unknown error".to_owned())))
    }

    /// Makes a single API request.
    async fn make_request(&self, user_prompt: &str) -> Result<String, SummaryError> {
        let system_message = ChatCompletionRequestSystemMessageArgs::default()
            .content(SYSTEM_PROMPT)
            .build()
            .map_err(|e| SummaryError::ApiError(e.to_string()))?;

        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content(user_prompt)
            .build()
            .map_err(|e| SummaryError::ApiError(e.to_string()))?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![
                ChatCompletionRequestMessage::System(system_message),
                ChatCompletionRequestMessage::User(user_message),
            ])
            .max_tokens(4096_u32)
            .build()
            .map_err(|e: async_openai::error::OpenAIError| SummaryError::ApiError(e.to_string()))?;

        let response = self.client.chat().create(request).await?;

        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| SummaryError::ApiError("Empty response from API".to_owned()))?;

        Ok(content)
    }
}

/// Formats the user prompt for the summarization request.
fn format_user_prompt(crate_name: &str, version: &str, docs_text: &str) -> String {
    format!(
        r#"Summarize the following Rust crate documentation for "{crate_name}" v{version}.

Create a summary suitable for an LLM to understand how to use this crate effectively.

---
DOCUMENTATION:

{docs_text}
---

Provide your summary in Markdown format."#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_user_prompt() {
        let prompt = format_user_prompt("test_crate", "1.0.0", "Some docs here");
        assert!(prompt.contains("test_crate"));
        assert!(prompt.contains("1.0.0"));
        assert!(prompt.contains("Some docs here"));
    }
}
