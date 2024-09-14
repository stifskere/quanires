use base64::{engine::general_purpose::URL_SAFE, Engine};
use reqwest::{get, header::{ACCEPT, CONTENT_TYPE, HOST, ORIGIN, PRAGMA, REFERER}, Client};
use rustc_hash::FxHashSet;
use scraper::{selectable::Selectable, Html, Selector};
use serde::Deserialize;
use urlencoding::encode;

#[derive(PartialEq, Eq, Hash, Clone, Default)]
pub struct AnimeEntry {
    name: String,
    url: String
}

impl AnimeEntry {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }
}

pub async fn query_anime(query: &str) -> Result<FxHashSet<AnimeEntry>, String> {
    get(format!("https://monoschinos2.com/buscar?q={}", encode(query)))
        .await
        .map_or_else(|err| Err(err.to_string()), |res| Ok(res.text()))?
        .await
        .map(|content| Html::parse_document(&content)
            .select(
                &Selector::parse("li.col.mb-5.ficha_efecto > article > a")
                    .unwrap()
            )
            .filter_map(|anime| Some(AnimeEntry {
                name: anime.select(&Selector::parse("h3").unwrap())
                    .next()?
                    .inner_html(),
                url: anime.attr("href")?
                    .to_string()
            }))
            .collect()
        )
        .map_err(|err| err.to_string())
}

#[derive(Deserialize)]
pub struct ChapterInfo {
    #[serde(rename = "episodio")]
    pub number: i32,
    pub url: String
}

impl ChapterInfo {
    pub fn number(&self) -> i32 {
        self.number
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Deserialize)]
struct ChapterResponse {
    pub caps: Vec<ChapterInfo>
}

pub async fn select_chapters(url: &str) -> Result<Vec<ChapterInfo>, String> {
    let client = Client::builder()
        .cookie_store(true)
        .build()
        .map_err(|err| err.to_string())?;

    let response = client.get(url)
        .send()
        .await
        .map_or_else(|err| Err(err.to_string()), |res| Ok(res.text()))?
        .await
        .map(|body| Html::parse_document(&body))
        .map_err(|err| err.to_string())?;

    let caps_url = response.select(
        &Selector::parse("section.caplist").unwrap()
    )
        .next()
        .and_then(|element| element.attr("data-ajax"))
        .ok_or("Couldn't get episode list URL.")?
        .to_string();

    let head_csrf = response.select(
        &Selector::parse("meta[name='csrf-token']").unwrap()
    )
        .next()
        .and_then(|element| element.attr("content"))
        .ok_or("Error retrieving CSRF token")?;

    let mut page_counter = 0;
    let mut result = Vec::new();

    loop {
        let chapters = client.post(caps_url.replace("ajax_pagination", "caplist"))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded; charset=UTF8")
            .header(HOST, "monoschinos2.com")
            .header(ORIGIN, "https://monoschinos2.com")
            .header(PRAGMA, "no-cache")
            .header(REFERER, url)
            .header(ACCEPT, "application/json, text/javascript, */*; q=0.01")
            .body(format!("_token={}&p={}", encode(head_csrf), page_counter))
            .send()
            .await
            .map_or_else(|err| Err(err.to_string()), |res| Ok(res.json::<ChapterResponse>()))?
            .await
            .map_err(|err| err.to_string())?;

        let length = chapters.caps.len();
        page_counter += 1;
        result.extend(chapters.caps);

        if length < 50 {
            break;
        }
    }

    Ok(result)
}

pub async fn get_play_links(url: &str) -> Result<FxHashSet<String>, String> {
    get(url)
        .await
        .map_or_else(|err| Err(err.to_string()), |res| Ok(res.text()))?
        .await
        .map_or_else(|err| Err(err.to_string()), |content| Html::parse_document(&content)
            .select(&Selector::parse("button.play-video").unwrap())
            .filter_map(|button| button.attr("data-player")
                .map(|attribute|
                    URL_SAFE.decode(attribute)
                        .map_err(|err| err.to_string())
                        .and_then(|decoded| String::from_utf8(decoded)
                            .map_err(|err| err.to_string())
                        )
                )
            )
            .collect()
        )
}
