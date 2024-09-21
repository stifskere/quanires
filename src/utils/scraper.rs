use base64::{engine::general_purpose::URL_SAFE, Engine};
use reqwest::{get, header::{ACCEPT, CONTENT_TYPE, HOST, ORIGIN, PRAGMA, REFERER}, Client, Error as ReqwestError};
use rustc_hash::FxHashSet;
use scraper::{selectable::Selectable, Html, Selector};
use serde::Deserialize;
use thiserror::Error;
use urlencoding::encode;

#[derive(PartialEq, Eq, Hash, Clone, Default)]
pub struct AnimeEntry {
    name: String,
    url: String
}

impl AnimeEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Error, Debug)]
pub enum QueryAnimeEror {
    #[error("No se pudo realizar la busqueda, hubo un error al hacer la solicitud: {0}")]
    Request(#[from] ReqwestError),

    #[error("La busqueda no obtuvo resultados.")]
    NoResults
}

pub async fn query_anime(query: &str) -> Result<FxHashSet<AnimeEntry>, QueryAnimeEror> {
    Ok(get(format!("https://monoschinos2.com/buscar?q={}", encode(query)))
        .await?
        .text()
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
            .collect::<FxHashSet<AnimeEntry>>()
        )?)
        .and_then(|res| if res.is_empty() { Err(QueryAnimeEror::NoResults) } else { Ok(res) })
}

#[derive(Deserialize, Clone)]
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

#[derive(Error, Debug)]
pub enum ChapterSelectionError {
    #[error("No se pudieron obtener los capitulos, hubo un error al realizar la solicitud: {0}.")]
    Request(#[from] ReqwestError),

    #[error("No se pudo obtener la URL de la lista de episodios.")]
    EpisodeListUrl,

    #[error("No se pudo obtener el token CSRF para la solicitud.")]
    Token
}

pub async fn select_chapters(url: &str) -> Result<Vec<ChapterInfo>, ChapterSelectionError> {
    let client = Client::builder()
        .cookie_store(true)
        .build()?;

    let response = client.get(url)
        .send()
        .await?
        .text()
        .await
        .map(|body| Html::parse_document(&body))?;

    let caps_url = response.select(
        &Selector::parse("section.caplist").unwrap()
    )
        .next()
        .and_then(|element| element.attr("data-ajax"))
        .ok_or(ChapterSelectionError::EpisodeListUrl)?
        .to_string();

    let head_csrf = response.select(
        &Selector::parse("meta[name='csrf-token']").unwrap()
    )
        .next()
        .and_then(|element| element.attr("content"))
        .ok_or(ChapterSelectionError::Token)?;

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
            .await?
            .json::<ChapterResponse>()
            .await?;

        let length = chapters.caps.len();
        page_counter += 1;
        result.extend(chapters.caps);

        if length < 50 {
            break;
        }
    }

    Ok(result)
}

#[derive(Error, Debug)]
pub enum PlayLinksError {
    #[error("No se pudieron obtener los enlaces, hubo un error con la solicitud.")]
    Request(#[from] ReqwestError),

    #[error("No se encontraron enlaces validos para este episodio.")]
    NoLinks
}

pub async fn get_play_links(url: &str) -> Result<FxHashSet<String>, PlayLinksError> {
    Ok(get(url)
        .await?
        .text()
        .await
        .map(|content| Html::parse_document(&content)
            .select(&Selector::parse("button.play-video").unwrap())
            .filter_map(|button| button.attr("data-player")
                .and_then(|attribute|
                    String::from_utf8(URL_SAFE.decode(attribute).ok()?).ok()
                )
            )
            .collect::<FxHashSet<String>>()
        )?)
        .and_then(|res| if res.is_empty() { Err(PlayLinksError::NoLinks) } else { Ok(res) })
}
