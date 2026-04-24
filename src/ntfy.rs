pub async fn send(
    http: &reqwest::Client,
    server: &str,
    topic: &str,
    title: &str,
    priority: &str,
    body: &str,
) -> reqwest::Result<()> {
    if topic.is_empty() {
        return Ok(());
    }
    http.post(format!("{}/{}", server.trim_end_matches('/'), topic))
        .header("Title", title)
        .header("Priority", priority)
        .body(body.to_owned())
        .send()
        .await?;
    Ok(())
}
