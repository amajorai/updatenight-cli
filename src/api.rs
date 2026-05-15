use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

pub fn api_base() -> String {
    std::env::var("UN_API_URL").unwrap_or_else(|_| "https://updatenight.com".to_string())
}

fn client_with_auth(token: Option<&str>) -> Client {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(tok) = token {
        if let Ok(v) = format!("Bearer {}", tok).parse() {
            headers.insert(reqwest::header::AUTHORIZATION, v);
        }
    }
    Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_default()
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Entry {
    pub kind: String,
    pub slug: String,
    pub name: String,
    pub tagline: String,
    pub pricing: Option<String>,
    pub categories: Vec<String>,
    #[serde(rename = "homepageUrl")]
    pub homepage_url: String,
    #[serde(rename = "installSnippet")]
    pub install_snippet: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "repoUrl")]
    pub repo_url: Option<String>,
    #[serde(rename = "docsUrl")]
    pub docs_url: Option<String>,
}

#[derive(Deserialize, Debug)]
struct EntriesResponse {
    pub items: Vec<Entry>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NewsItem {
    pub title: String,
    pub summary: String,
    #[serde(rename = "sourceName")]
    pub source_name: String,
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    pub topics: Vec<String>,
    #[serde(rename = "postedAt")]
    pub posted_at: String,
}

#[derive(Deserialize, Debug)]
struct NewsResponse {
    pub items: Vec<NewsItem>,
}

pub async fn search_entries(query: &str, token: Option<&str>) -> Result<Vec<Entry>> {
    let base = api_base();
    let client = client_with_auth(token);
    let resp: EntriesResponse = client
        .get(format!("{}/api/entries", base))
        .query(&[("q", query), ("status", "published"), ("limit", "20")])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.items)
}

pub async fn list_by_category(
    kind: &str,
    category: &str,
    token: Option<&str>,
) -> Result<Vec<Entry>> {
    let base = api_base();
    let client = client_with_auth(token);
    let resp: EntriesResponse = client
        .get(format!("{}/api/entries", base))
        .query(&[
            ("kind", kind),
            ("category", category),
            ("status", "published"),
            ("limit", "20"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.items)
}

pub async fn get_news(days: u32, token: Option<&str>) -> Result<Vec<NewsItem>> {
    let base = api_base();
    let client = client_with_auth(token);
    let days_str = days.to_string();
    let resp: NewsResponse = client
        .get(format!("{}/api/news", base))
        .query(&[("days", days_str.as_str()), ("status", "published"), ("limit", "30")])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.items)
}

pub async fn semantic_search(query: &str, token: &str) -> Result<Vec<Entry>> {
    let base = api_base();
    let client = client_with_auth(Some(token));
    let resp: EntriesResponse = client
        .post(format!("{}/api/search", base))
        .json(&serde_json::json!({ "q": query, "limit": 20 }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(resp.items)
}
