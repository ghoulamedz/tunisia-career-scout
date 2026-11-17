//! Gmail API
//!
//! High-level Gmail API operations.

use anyhow::Result;
use google_gmail1::Gmail;
use google_gmail1::api::Message;
use crate::models::source::{AuthData, SourceConfig};

pub struct GmailApi {
    gmail: Gmail<reqwest::Client>,
}

impl GmailApi {
    pub fn new(gmail: Gmail<reqwest::Client>) -> Self {
        Self { gmail }
    }

    pub async fn list_messages(
        &self,
        label_ids: &[String],
        query: Option<&str>,
        max_results: u32,
    ) -> Result<Vec<Message>> {
        let mut q = String::new();
        for label in label_ids {
            q.push_str(&format!("label:{} ", label));
        }
        if let Some(query) = query {
            q.push_str(query);
        }
        q.push_str(" newer_than:30d");

        let response = self.gmail.users().messages()
            .list("me")
            .q(q)
            .max_results(max_results as i32)
            .doit()
            .await?;

        Ok(response.1.messages.unwrap_or_default())
    }

    pub async fn fetch_message(&self, msg_id: &str) -> Result<Message> {
        let response = self.gmail.users().messages()
            .get("me", msg_id)
            .format("full")
            .doit()
            .await?;
        Ok(response.1)
    }

    pub async fn modify_labels(&self, msg_id: &str, add_labels: &[String], remove_labels: &[String]) -> Result<()> {
        let mut modify = google_gmail1::api::ModifyMessageRequest::default();
        modify.add_label_ids = Some(add_labels.to_vec());
        modify.remove_label_ids = Some(remove_labels.to_vec());

        self.gmail.users().messages()
            .modify("me", msg_id, modify)
            .doit()
            .await?;
        Ok(())
    }

    pub async fn create_draft(&self, message: Message) -> Result<()> {
        let mut draft = google_gmail1::api::Draft::default();
        draft.message = Some(message);

        self.gmail.users().drafts()
            .create("me", draft)
            .doit()
            .await?;
        Ok(())
    }
}

pub fn message_to_raw_content(message: &Message) -> crate::scrapers::pipeline::RawContent {
    let headers = message.payload.as_ref()
        .and_then(|p| p.headers.as_ref())
        .map(|h| h.iter().map(|hv| (hv.name.clone(), hv.value.clone())).collect::<Vec<_>>())
        .unwrap_or_default();

    let get_header = |name: &str| -> Option<String> {
        headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.clone())
    };

    let subject = get_header("Subject").unwrap_or_default();
    let from = get_header("From").unwrap_or_default();
    let date = get_header("Date").unwrap_or_default();
    let message_id = get_header("Message-ID").unwrap_or_default();

    // Extract body
    let body = extract_body(message.payload.as_ref());

    let content_hash = format!("{:x}", md5::compute(&body));

    let metadata = serde_json::json!({
        "subject": subject,
        "from": from,
        "date": date,
        "message_id": message_id,
        "html_body": body,
        "headers": headers.into_iter().collect::<serde_json::Map<_, _>>(),
    });

    crate::scrapers::pipeline::RawContent {
        source_url: format!("gmail:{}", message_id),
        content: body,
        content_type: crate::scrapers::pipeline::ContentType::Email,
        fetched_at: chrono::DateTime::parse_from_rfc2822(&date)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or(chrono::Utc::now()),
        content_hash,
        metadata,
    }
}

fn extract_body(payload: Option<&google_gmail1::api::MessagePart>) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    if let Some(part) = payload {
        if part.mime_type == Some("text/html".to_string()) {
            if let Some(data) = part.body.as_ref().and_then(|b| b.data.as_ref()) {
                return URL_SAFE_NO_PAD.decode(data)
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .unwrap_or_default();
            }
        }
        if part.mime_type == Some("text/plain".to_string()) {
            if let Some(data) = part.body.as_ref().and_then(|b| b.data.as_ref()) {
                return URL_SAFE_NO_PAD.decode(data)
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .unwrap_or_default();
            }
        }
        if let Some(parts) = part.parts.as_ref() {
            for p in parts {
                let body = extract_body(Some(p));
                if !body.is_empty() {
                    return body;
                }
            }
        }
    }
    String::new()
}

pub async fn fetch_messages(
    oauth: &crate::gmail::oauth::GmailOAuth,
    config: &SourceConfig,
) -> Result<Vec<crate::scrapers::pipeline::RawContent>> {
    let access_token = oauth.get_valid_access_token().await?;
    
    let max_results = match config {
        SourceConfig::Gmail { max_results, .. } => *max_results,
        _ => Some(50),
    }.unwrap_or(50);

    let label_ids = match config {
        SourceConfig::Gmail { label_ids, .. } => label_ids,
        _ => &[],
    };

    let query = match config {
        SourceConfig::Gmail { query, .. } => query.as_deref(),
        _ => None,
    };

    let messages = fetch_messages_with_token(&access_token, label_ids, query, max_results).await?;
    let mut results = Vec::new();

    for msg in messages {
        if let Some(id) = msg.id {
            if let Ok(full_msg) = fetch_message_with_token(&access_token, &id).await {
                results.push(message_to_raw_content(&full_msg));
            }
        }
    }

    Ok(results)
}

async fn fetch_messages_with_token(
    access_token: &str,
    label_ids: &[String],
    query: Option<&str>,
    max_results: u32,
) -> Result<Vec<google_gmail1::api::Message>> {
    let client = reqwest::Client::new();
    let mut q = String::new();
    for label in label_ids {
        q.push_str(&format!("label:{} ", label));
    }
    if let Some(query) = query {
        q.push_str(query);
    }
    q.push_str(" newer_than:30d");

    let response = client
        .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
        .bearer_auth(access_token)
        .query(&[("q", q), ("maxResults", max_results.to_string())])
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    let messages = json["messages"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(|s| {
                    let mut msg = google_gmail1::api::Message::default();
                    msg.id = Some(s.to_string());
                    msg
                }))
                .collect()
        })
        .unwrap_or_default();

    Ok(messages)
}

async fn fetch_message_with_token(
    access_token: &str,
    msg_id: &str,
) -> Result<google_gmail1::api::Message> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{}", msg_id))
        .bearer_auth(access_token)
        .query(&[("format", "full")])
        .send()
        .await?;

    let mut message: google_gmail1::api::Message = response.json().await?;
    Ok(message)
}