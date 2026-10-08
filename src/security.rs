pub fn terminal_text(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_control()).collect()
}

pub fn http_url(value: &str) -> anyhow::Result<reqwest::Url> {
    anyhow::ensure!(
        !value.chars().any(char::is_control),
        "URL contains control characters"
    );
    let url = reqwest::Url::parse(value)?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "Expected an HTTP(S) URL without credentials"
    );
    Ok(url)
}

pub fn open_url(value: &str) -> anyhow::Result<()> {
    let url = http_url(value)?;
    open::that_detached(url.as_str())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_urls_are_web_urls_and_terminal_controls_are_removed() {
        for url in [
            "https://example.com/a?x=%22%26",
            "http://localhost:3000/verify",
        ] {
            assert!(http_url(url).is_ok());
        }
        for url in [
            "file:///tmp/x",
            "https://user:pass@example.com",
            "https://example.com/\n",
            "javascript:alert(1)",
        ] {
            assert!(http_url(url).is_err());
        }
        assert_eq!(
            terminal_text("hello\x1b[31m\x07\r\u{009b}世界"),
            "hello[31m世界"
        );
    }
}
