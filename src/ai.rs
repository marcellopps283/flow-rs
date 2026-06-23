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

use parakeet_rs::{Parakeet, ExecutionConfig, ExecutionProvider, Transcriber};
use std::path::Path;

pub struct AiPipeline {
    client: Client,
    api_key: String,
    parakeet: Parakeet,
}

impl AiPipeline {
    pub fn new() -> Result<Self, anyhow::Error> {
        let api_key = std::env::var("GROQ_API_KEY")
            .unwrap_or_else(|_| "your_api_key_here".to_string());
        
        let config = ExecutionConfig::new()
            .with_execution_provider(ExecutionProvider::Cuda);
            
        let model_dir = Path::new("models/nemotron");
        if !model_dir.exists() {
            println!("WARNING: Nemotron model directory not found at {:?}", model_dir);
        }
        
        let parakeet = Parakeet::from_pretrained(model_dir.to_str().unwrap(), Some(config))
            .map_err(|e| anyhow::anyhow!("Failed to load Parakeet/Nemotron model: {}", e))?;

        Ok(Self {
            client: Client::new(),
            api_key,
            parakeet,
        })
    }

    /// Transcribe audio using local NVIDIA Nemotron model via parakeet-rs
    pub fn transcribe_audio(&mut self, audio_buffer: &[f32], sample_rate: u32, channels: u16) -> Result<String, anyhow::Error> {
        // Transcribe the raw f32 samples using Parakeet
        match self.parakeet.transcribe_samples(audio_buffer.to_vec(), sample_rate, channels, None) {
            Ok(result) => Ok(result.text),
            Err(e) => Err(anyhow::anyhow!("Transcription error: {}", e))
        }
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
