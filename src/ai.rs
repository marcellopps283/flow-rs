use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    temperature: f32,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

pub struct AiPipeline {
    client: Client,
    api_key: String,
}

impl AiPipeline {
    pub fn new() -> Result<Self, anyhow::Error> {
        // Load API key from environment, with a fallback for local testing
        let api_key = std::env::var("GROQ_API_KEY")
            .unwrap_or_else(|_| "your_api_key_here".to_string());
        
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    /// Transcribe audio using local NVIDIA Parakeet/Nemotron model via ONNX Runtime
    pub fn transcribe_audio(&self, _audio_buffer: &[f32]) -> Result<String, anyhow::Error> {
        // TODO: Initialize `ort::Session` and run local inference with NVIDIA weights
        // Simulated response for UI testing
        Ok("Simulated transcription from the local NVIDIA Nemotron ASR model.".to_string())
    }

    /// Polish text using Groq's Llama 3.3 model
    pub async fn polish_text(&self, raw_text: &str) -> Result<String, anyhow::Error> {
        let req_body = GroqRequest {
            model: "llama-3.3-70b-versatile".to_string(),
            messages: vec![
                GroqMessage {
                    role: "system".to_string(),
                    content: "You are an expert dictation assistant. Fix grammar, remove filler words (ums, ahs), and format the text properly. Output ONLY the polished text without quotes or explanations.".to_string(),
                },
                GroqMessage {
                    role: "user".to_string(),
                    content: raw_text.to_string(),
                }
            ],
            temperature: 0.1,
        };

        let response = self.client.post("https://api.groq.com/openai/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Groq API Error: {}", error_text));
        }

        let resp_json: GroqResponse = response.json().await?;
        
        if let Some(choice) = resp_json.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow::anyhow!("No choices returned from Groq"))
        }
    }
}
