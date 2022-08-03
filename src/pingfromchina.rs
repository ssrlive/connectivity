use scraper::{Html, Selector};
use std::collections::HashMap;

pub async fn ping_from_china(host: &str, port: u16) -> bool {
    let url = format!("https://tool.chinaz.com/port?host={}&port={}", host, port);

    let resp = reqwest::get(url).await.unwrap();
    let text = resp.text().await.unwrap();

    let mut encode = None;
    {
        let document = Html::parse_document(&text);
        let selector = Selector::parse(r#"input"#).unwrap();
        for item in document.select(&selector) {
            let value = item.value();
            if let Some(val) = value.attr("id") && val == "encode" {
                    if let Some(val) = value.attr("value") {
                        encode = Some(val.to_string());
                        break;
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
        let resp = client.post(url).form(&map).send().await.unwrap();

        let text = resp.text().await.unwrap();
        result = text.contains("status:1");
    }
    result
}
