use crate::provider::{LLMProviderTrait, LLMRequest, LLMResponse};
use async_trait::async_trait;

/// Mock LLM provider for offline testing and response injection
#[derive(Debug, Clone)]
pub struct MockLLMProvider {
    /// Predefined response to return (for testing)
    response: Option<LLMResponse>,
    /// Artificial delay in milliseconds (for timeout testing)
    delay_ms: Option<u64>,
}

impl MockLLMProvider {
    /// Create a new MockLLMProvider with no predefined response
    ///
    /// Default behavior: returns a simple response indicating this is a mock
    pub fn new() -> Self {
        Self {
            response: None,
            delay_ms: None,
        }
    }

    /// Create a MockLLMProvider with a predefined response
    ///
    /// Useful for testing specific scenarios
    pub fn with_response(response: LLMResponse) -> Self {
        Self {
            response: Some(response),
            delay_ms: None,
        }
    }

    /// Set an artificial delay in milliseconds before returning response
    ///
    /// Useful for testing timeout scenarios
    pub fn with_delay(&self, delay_ms: u64) -> Self {
        Self {
            response: self.response.clone(),
            delay_ms: Some(delay_ms),
        }
    }

    /// Clear the artificial delay
    pub fn clear_delay(&mut self) {
        self.delay_ms = None;
    }

    /// Update the predefined response
    pub fn set_response(&mut self, response: LLMResponse) {
        self.response = Some(response);
    }

    /// Update the predefined response with delay
    pub fn set_response_with_delay(&mut self, response: LLMResponse, delay_ms: u64) {
        self.response = Some(response);
        self.delay_ms = Some(delay_ms);
    }

    /// Clear the predefined response (revert to default behavior)
    pub fn clear_response(&mut self) {
        self.response = None;
    }
}

impl Default for MockLLMProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProviderTrait for MockLLMProvider {
    async fn complete(&self, _request: &LLMRequest) -> anyhow::Result<LLMResponse> {
        if let Some(delay_ms) = self.delay_ms {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }

        if let Some(ref response) = self.response {
            Ok(response.clone())
        } else {
            Ok(LLMResponse {
                content: "Mock LLM response - for testing purposes".to_string(),
                tokens_used: 100,
                suggested_tasks: vec![],
                commands: vec![],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::SuggestedTask;

    #[tokio::test]
    async fn test_mock_llm_provider_default_response() {
        let provider = MockLLMProvider::new();
        let request = LLMRequest {
            prd: "Test PRD".to_string(),
            task: "Test task".to_string(),
            context: vec![],
            max_tokens: Some(100),
        };

        let result = provider.complete(&request).await.unwrap();

        assert_eq!(result.content, "Mock LLM response - for testing purposes");
        assert_eq!(result.tokens_used, 100);
        assert!(result.suggested_tasks.is_empty());
        assert!(result.commands.is_empty());
    }

    #[tokio::test]
    async fn test_mock_llm_provider_returns_injected_response() {
        let injected_response = LLMResponse {
            content: "Injected test response".to_string(),
            tokens_used: 500,
            suggested_tasks: vec![SuggestedTask {
                title: "Test task".to_string(),
                description: "Test description".to_string(),
                priority: 10,
            }],
            commands: vec!["echo test".to_string()],
        };

        let provider = MockLLMProvider::with_response(injected_response.clone());

        let request = LLMRequest {
            prd: "Test PRD".to_string(),
            task: "Test task".to_string(),
            context: vec![],
            max_tokens: Some(100),
        };

        let result = provider.complete(&request).await.unwrap();

        assert_eq!(result.content, "Injected test response");
        assert_eq!(result.tokens_used, 500);
        assert_eq!(result.suggested_tasks.len(), 1);
        assert_eq!(result.suggested_tasks[0].title, "Test task");
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.commands[0], "echo test");
    }

    #[tokio::test]
    async fn test_mock_llm_provider_set_and_clear_response() {
        let mut provider = MockLLMProvider::new();

        // First call should return default response
        let request = LLMRequest {
            prd: "Test".to_string(),
            task: "Test".to_string(),
            context: vec![],
            max_tokens: None,
        };

        let result1 = provider.complete(&request).await.unwrap();
        assert_eq!(result1.content, "Mock LLM response - for testing purposes");

        // Set custom response
        provider.set_response(LLMResponse {
            content: "Custom response".to_string(),
            tokens_used: 200,
            suggested_tasks: vec![],
            commands: vec![],
        });

        let result2 = provider.complete(&request).await.unwrap();
        assert_eq!(result2.content, "Custom response");
        assert_eq!(result2.tokens_used, 200);

        // Clear response
        provider.clear_response();

        let result3 = provider.complete(&request).await.unwrap();
        assert_eq!(result3.content, "Mock LLM response - for testing purposes");
    }

    #[test]
    fn test_mock_llm_provider_default() {
        let provider = MockLLMProvider::default();
        assert!(provider.response.is_none());
    }

    #[test]
    fn test_mock_llm_provider_clone() {
        let injected = LLMResponse {
            content: "Test".to_string(),
            tokens_used: 100,
            suggested_tasks: vec![],
            commands: vec![],
        };

        let provider1 = MockLLMProvider::with_response(injected);
        let provider2 = provider1.clone();

        // Both should work independently
        assert!(provider1.response.is_some());
        assert!(provider2.response.is_some());
    }
}
