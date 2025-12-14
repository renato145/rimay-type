use reqwest::multipart;
use serde::Deserialize;

pub struct GroqClient {
    client: reqwest::Client,
    api_key: String,
}

impl GroqClient {
    pub fn new(api_key: &str) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            api_key: api_key.to_string(),
        }
    }

    pub async fn transcribe(&self, wav_bytes: Vec<u8>) -> anyhow::Result<String> {
        let file_part = multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;
        let form = multipart::Form::new()
            .part("file", file_part)
            .text("model", "whisper-large-v3-turbo")
            .text("temperature", "0")
            .text("response_format", "json");
        let text = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?
            .json::<GroqResult>()
            .await?
            .text;
        Ok(text.trim().to_string())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct GroqResult {
    text: String,
    #[allow(dead_code)]
    x_groq: serde_json::Value,
}
