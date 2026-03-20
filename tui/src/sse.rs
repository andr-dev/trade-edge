use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use trade_edge_core::SseEvent;

pub async fn sse_stream(url: String, tx: mpsc::UnboundedSender<SseEvent>) {
    let client = reqwest::Client::new();
    loop {
        match connect_sse(&client, &url, &tx).await {
            Ok(()) => break,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

async fn connect_sse(
    client: &reqwest::Client,
    url: &str,
    tx: &mpsc::UnboundedSender<SseEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = client.get(url).send().await?;
    let mut stream = response.bytes_stream();

    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text =
            std::str::from_utf8(&chunk).map_err(|e| format!("invalid UTF-8 in SSE stream: {e}"))?;
        buf.push_str(text);

        while let Some(pos) = buf.find("\n\n") {
            let frame = &buf[..pos];

            for line in frame.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if let Ok(event) = serde_json::from_str::<SseEvent>(data) {
                        if tx.send(event).is_err() {
                            return Ok(());
                        }
                    }
                }
            }

            let end = pos + 2;
            buf.drain(..end);
        }
    }
    Ok(())
}
