use crate::ChinazResult;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::error::Error;

pub async fn ping_from_china(host: &str, port: u16) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let url = format!("https://tool.chinaz.com/port?host={}&port={}", host, port);

    let resp = reqwest::get(url).await?;
    let text = resp.text().await?;

    let mut token = None;
    let mut ts = None;
    {
        let document = Html::parse_document(&text);
        if let Ok(selector) = Selector::parse(r#"input"#) {
            for item in document.select(&selector) {
                let value = item.value();
                if let Some("token") = value.attr("id") {
                    if let Some(val) = value.attr("value") {
                        token = Some(val.to_string());
                    }
                } else if let Some("ts") = value.attr("id") {
                    if let Some(val) = value.attr("value") {
                        ts = Some(val.to_string());
                    }
                }
            }
        }
    }
    if ts.is_none() || token.is_none() {
        return Ok(false);
    }
    let mut result = false;
    //if let Some(token) = token && let Some(ts) = ts {
    let token = token.unwrap();
    let ts = ts.unwrap();
    {
        let url = "https://tool.chinaz.com/scanport";
        let map = HashMap::from([
            ("token", token),
            ("ts", ts),
            ("host", host.to_string()),
            ("port", port.to_string()),
        ]);

        let client = reqwest::Client::new();
        let resp = client.post(url).form(&map).send().await?;

        let text = resp.text().await?;
        let json = serde_json::from_str::<ChinazResult>(&text).unwrap();
        if json.code == 1 {
            result = json.data.is_open;
        }
    }
    Ok(result)
}
