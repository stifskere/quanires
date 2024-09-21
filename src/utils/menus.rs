use super::{scraper::{query_anime, select_chapters, AnimeEntry, ChapterInfo, ChapterSelectionError, QueryAnimeEror}, tracker::{EpisodeTracker, TrackerError}};
use promptuity::{prompts::{Input, Select, SelectOption}, Error as PromptuityError, Promptuity};
use reqwest::Error as ReqwestError;
use thiserror::Error;
use std::{io::Stderr, process::exit, sync::LazyLock};

fn start
(prompt: &mut Promptuity<'_, Stderr>, title: &str)
-> Result<(), PromptuityError> {
    prompt.term().clear()?;
    prompt.with_intro(title).begin()?;

    Ok(())
}

#[derive(Error, Debug)]
pub enum QueryMenuError {
    #[error("Hubo un error al mostrar el menu.")]
    Prompt(#[from] PromptuityError),

    #[error("Hubo un error al hacer la solicitud.")]
    Request(ReqwestError),

    #[error("{0}")]
    Next(String) // This is done due to recurse indirection
    // it is like this in both places to mantain consistency.
}

// TODO: implement episode tracker

pub async fn query_menu
(prompt: &mut Promptuity<'_, Stderr>, gen_start: bool)
-> Result<(), QueryMenuError> {
    if gen_start {
        start(prompt, "Bienvenido a quanires!")?;
    }

    let mut first_question = "Que deseas ver?";
    let query_result;

    loop {
        match query_anime(&prompt.prompt(&mut Input::new(first_question))?).await {
            Ok(res) => {
                query_result = res;
                break;
            },
            Err(QueryAnimeEror::Request(err)) => return Err(QueryMenuError::Request(err)),
            Err(QueryAnimeEror::NoResults) => {
                prompt.error("No se encontraron resultados,")?;
                first_question = "prueba a buscar otra cosa. Que deseas ver?"
            }
        };
    }

    let options_result = prompt.prompt(&mut Select::new(
        "Tu busqueda obvtuvo esos resultados.",
        {
            let mut options = query_result
                .iter()
                .map(|result| SelectOption::new(result.name(), result.url()))
                .collect::<Vec<SelectOption<&str>>>();

            options.extend(
                vec![
                    SelectOption::new("Volver a buscar", "op_retry").with_hint("Realiza otra busqueda."),
                    SelectOption::new("Salir", "op_quit").with_hint("Cerrar el programa.")
                ]
            );

            options
        }
    ))?;

    match options_result {
        "op_quit" => {
            prompt.with_outro("Adios!").finish()?;
            exit(0);
        },
        "op_retry" => {
            Box::pin(query_menu(prompt, false)).await
        },
        anime_url => {
            Box::pin(chapter_menu(
                prompt,
                false,
                query_result
                    .iter()
                    .find(|result| result.url() == anime_url)
                    .unwrap()
            ))
                .await
                .map_err(|err| QueryMenuError::Next(err.to_string()))
        }
    }
}

#[derive(Error, Debug)]
pub enum ChapterMenuError {
    #[error("Hubo un error al mostrar el menu.")]
    Prompt(#[from] PromptuityError),

    #[error("Hubo un error al seleccionar los capitulos.")]
    ChapterSelection(#[from] ChapterSelectionError),

    #[error("{0}")]
    Last(String)
}

pub struct ChapterSelection {
    last: Option<ChapterInfo>,
    current: ChapterInfo,
    next: Option<ChapterInfo>
}

pub async fn chapter_menu
(prompt: &mut Promptuity<'_, Stderr>, gen_start: bool, anime: &AnimeEntry)
-> Result<(), ChapterMenuError> {
    if gen_start {
        start(prompt, anime.name())?;
    }

    let chapters = select_chapters(anime.url()).await?;

    let selection = prompt.prompt(&mut Select::new(
        "Que capitulo deseas ver?",
        {
            let mut options = vec![
                SelectOption::new("Atras", "op_back").with_hint("Realiza otra busqueda."),
                SelectOption::new("Salir", "op_quit").with_hint("Cerrar el programa.")
            ];

            options.extend(
                chapters
                    .iter()
                    .map(|chapter| SelectOption::new(
                        format!("Capitulo {}", chapter.number()),
                        chapter.url()
                    ))
                    .collect::<Vec<SelectOption<_>>>()
            );

            options
        }
    ))?;

    match selection {
        "op_quit" => {
            prompt.with_outro("Adios!").finish()?;
            exit(0);
        },
        "op_back" => {
            Box::pin(query_menu(prompt, true))
                .await
                .map_err(|err| ChapterMenuError::Last(err.to_string()))
        },
        chapter => {
            let current = chapters
                .as_slice()
                .iter()
                .find(|possible| possible.url() == chapter)
                .unwrap();

            let last = chapters
                .as_slice()
                .iter()
                .find(|possible| possible.number() == current.number() - 1)
                .cloned();

            let next = chapters
                .as_slice()
                .iter()
                .find(|possible| possible.number == current.number() + 1)
                .cloned();

            Ok(Box::pin(play_menu(
                prompt,
                anime,
                &ChapterSelection {
                    last: last.clone(),
                    current: current.clone(),
                    next: next.clone()
                }
            )).await?)
        }
    }
}

pub async fn play_menu
(prompt: &mut Promptuity<'_, Stderr>, anime: &AnimeEntry, chapter: &ChapterSelection)
-> Result<(), PromptuityError> {
    start(prompt, &format!("{} | Capitulo {}", anime.name(), chapter.current.number()))?;
        

    Ok(())
}
