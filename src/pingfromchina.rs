use scraper::{Html, Selector};
use std::collections::HashMap;
use std::error::Error;

pub async fn ping_from_china(host: &str, port: u16) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let url = format!("https://tool.chinaz.com/port?host={}&port={}", host, port);

    let resp = reqwest::get(url).await?;
    let text = resp.text().await?;

    let mut encode = None;
    {
        let document = Html::parse_document(&text);
        if let Ok(selector) = Selector::parse(r#"input"#) {
            for item in document.select(&selector) {
                let value = item.value();
                if let Some("encode") = value.attr("id") {
                    if let Some(val) = value.attr("value") {
                        encode = Some(val.to_string());
                        break;
                    }
                }
            }
        }
    }
    let mut result = false;
    if let Some(encode) = encode {
        let url = "https://tool.chinaz.com/iframe.ashx?t=port";
        let map = HashMap::from([
            ("encode", encode),
            ("host", host.to_string()),
            ("port", port.to_string()),
        ]);

        let client = reqwest::Client::new();
        let resp = client.post(url).form(&map).send().await?;

        let text = resp.text().await?;
        result = text.contains("status:1");
    }
    Ok(result)
}
