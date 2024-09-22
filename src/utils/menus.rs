use super::{mpv::{check_mpv, run_mpv}, scraper::{get_play_links, query_anime, select_chapters, AnimeEntry, ChapterInfo, ChapterSelectionError, QueryAnimeEror}, tracker::{EpisodeTracker, TrackerError}};
use promptuity::{prompts::{Input, Select, SelectOption}, Error as PromptuityError, Promptuity};
use reqwest::Error as ReqwestError;
use thiserror::Error;
use std::{io::Stderr, process::exit};

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

pub async fn query_menu
(
    prompt: &mut Promptuity<'_, Stderr>,
    gen_start: bool,
    tracker: &mut Option<&mut EpisodeTracker>
)
-> Result<(), QueryMenuError> {
    if gen_start {
        start(prompt, "Bienvenido a quanires!")?;
    }

    if !check_mpv() {
        prompt.error("No se encontro MPV.")?;
        prompt.with_outro("Instalalo antes de continuar.").finish()?;
        exit(-1);
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
                    SelectOption::new("Volver a buscar", "op_retry").with_hint("Realiza otra busqueda"),
                    SelectOption::new("Salir", "op_quit").with_hint("Cerrar el programa")
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
            Box::pin(query_menu(prompt, false, tracker)).await
        },
        anime_url => {
            Box::pin(chapter_menu(
                prompt,
                false,
                query_result
                    .iter()
                    .find(|result| result.url() == anime_url)
                    .unwrap(),
                tracker
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
    Last(String),

    #[error("Error del tracker: {0}")]
    TrackerError(#[from] TrackerError)
}

pub struct ChapterSelection {
    last: Option<ChapterInfo>,
    current: ChapterInfo,
    next: Option<ChapterInfo>
}

pub async fn chapter_menu
(
    prompt: &mut Promptuity<'_, Stderr>,
    gen_start: bool,
    anime: &AnimeEntry,
    tracker: &mut Option<&mut EpisodeTracker>
)
-> Result<(), ChapterMenuError> {
    if gen_start {
        start(prompt, anime.name())?;
    }

    let chapters = select_chapters(anime.url()).await?;

    let selection = prompt.prompt(&mut Select::new(
        "Que capitulo deseas ver?",
        {
            let mut options = vec![
                SelectOption::new("Atras", "op_back").with_hint("Realiza otra busqueda"),
                SelectOption::new("Salir", "op_quit").with_hint("Cerrar el programa")
            ];

            options.extend(
                chapters
                    .iter()
                    .map(|chapter| {
                        let mut option = SelectOption::new(
                            format!("Capitulo {}", chapter.number()),
                            chapter.url()
                        );

                        if let Some(ref mut tracker) = tracker {
                            if tracker.episode_is_seen(anime.url(), &chapter.number()) {
                                option = option.with_hint("Visto")
                            }
                        }

                        option
                    })
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
            Box::pin(query_menu(prompt, true, tracker))
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

            if let Some(ref mut tracker) = tracker {
                tracker.watch_episode(anime.url(), current.number())?;
            }

            Ok(Box::pin(play_menu(
                prompt,
                anime,
                &ChapterSelection {
                    last: last.clone(),
                    current: current.clone(),
                    next: next.clone()
                },
                tracker
            )).await?)
        }
    }
}

pub async fn play_menu
(
    prompt: &mut Promptuity<'_, Stderr>,
    anime: &AnimeEntry,
    chapter: &ChapterSelection,
    tracker: &mut Option<&mut EpisodeTracker>
)
-> Result<(), PromptuityError> {
    start(prompt, &format!("{} | Capitulo {}", anime.name(), chapter.current.number()))?;

    let mut options = Vec::new();

    if chapter.last.is_some() {
        options.push(SelectOption::new("Anterior episodio", "op_last_episode"));
    }

    if chapter.next.is_some() {
        options.push(SelectOption::new("Siguiente episodio", "op_next_episode"));
    }

    if let Some(ref mut tracker) = tracker {
        if tracker.episode_is_seen(anime.url(), &chapter.current.number()) {
            options.push(SelectOption::new("Desmarcar como visto", "op_unwatch"));
        }
    }

    options.extend([
        SelectOption::new("Ver otro anime", "op_watch_other_anime"),
        SelectOption::new("Ver otro capitulo", "op_watch_other_episode"),
        SelectOption::new("Salir", "op_exit").with_hint("Cerrar el programa y el reproductor")
    ]);

    match get_play_links(chapter.current.url()).await {
        Ok(links) => {
            let _tx = run_mpv(anime.name().to_string(), chapter.current.number(), links);

            // TODO: implement listening to TX.
        },
        Err(err) => {
            prompt.error(format!("No se pudieron obtener los links: {err}"))?;
        }
    }

    // TODO: implement menu handling along with possible error returned from TX.

    Ok(())
}
