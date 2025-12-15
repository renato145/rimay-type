use anyhow::Context;
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

    #[tracing::instrument(skip_all, fields(%opts), ret)]
    pub async fn transcribe(
        &self,
        wav_bytes: Vec<u8>,
        opts: TranscribeOpts,
    ) -> anyhow::Result<String> {
        let form = opts.form(wav_bytes)?;
        let response = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;
        if response.status().is_success() {
            let raw = response
                .json::<serde_json::Value>()
                .await
                .context("Failed to get response.")?;
            let text = serde_json::from_value::<GroqResult>(raw.clone())
                .inspect_err(|_| tracing::info!(?raw, "Raw response."))
                .context("Failed to deserialize GroqResult.")?
                .text;
            Ok(text.trim().to_string())
        } else {
            let e = response
                .text()
                .await
                .unwrap_or_else(|e| format!("Failed to read error response: {e}"));
            anyhow::bail!("Groq error: {e}");
        }
    }
}

#[derive(Clone, Debug)]
pub struct TranscribeOpts {
    /// Required ID of the model to use ("whisper-large-v3-turbo" or "whisper-large-v3").
    pub model: String,
    /// The language of the input audio. Supplying the input language in ISO-639-1 (i.e. en, tr`)
    /// format will improve accuracy and latency.
    pub language: Option<String>,
    /// Prompt to guide the model's style or specify how to spell unfamiliar words. (limited to 224
    /// tokens)
    pub prompt: Option<String>,
}

impl TranscribeOpts {
    fn form(self, wav_bytes: Vec<u8>) -> anyhow::Result<multipart::Form> {
        let file_part = multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;
        let mut form = multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model)
            .text("temperature", "0")
            .text("response_format", "json");
        if let Some(language) = self.language {
            form = form.text("language", language);
        }
        if let Some(prompt) = self.prompt {
            form = form.text("prompt", prompt);
        }
        Ok(form)
    }
}

impl std::fmt::Display for TranscribeOpts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "model={:?}", self.model)?;
        if let Some(language) = &self.language {
            write!(f, ", language={language:?}")?;
        }
        if let Some(prompt) = &self.prompt {
            let preview = prompt
                .chars()
                .take(10)
                .chain("...".chars())
                .collect::<String>();
            write!(f, ", preview={preview:?}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct GroqResult {
    text: String,
    #[allow(dead_code)]
    x_groq: serde_json::Value,
}
